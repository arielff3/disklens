//! Store (MSIX) apps, through PowerShell.

use serde::Deserialize;
use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Main, removable, non-system packages that have a Start menu entry (whose name we use).
const LIST_SCRIPT: &str = r#"
$names = @{}
Get-StartApps | ForEach-Object {
  $family = ($_.AppID -split '!')[0]
  if (-not $names.ContainsKey($family)) { $names[$family] = $_.Name }
}
Get-AppxPackage -PackageTypeFilter Main |
  Where-Object { -not $_.IsFramework -and -not $_.NonRemovable -and $_.SignatureKind -ne 'System' -and $names.ContainsKey($_.PackageFamilyName) } |
  ForEach-Object {
    [pscustomobject]@{
      fullName = $_.PackageFullName
      family = $_.PackageFamilyName
      name = $names[$_.PackageFamilyName]
      version = [string]$_.Version
      location = $_.InstallLocation
      publisher = $_.Publisher
    }
  } | ConvertTo-Json -Compress
"#;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub full_name: String,
    pub family: String,
    pub name: String,
    pub version: Option<String>,
    pub location: Option<String>,
    /// A certificate subject such as `CN=…, O=Spotify AB, …`.
    pub publisher: Option<String>,
}

impl Package {
    /// "O=Spotify AB" from the certificate subject, else its CN.
    pub fn publisher_name(&self) -> Option<String> {
        let subject = self.publisher.as_deref()?;
        let field = |key: &str| {
            subject
                .split(',')
                .map(str::trim)
                .find_map(|part| part.strip_prefix(key))
                .map(|v| v.trim_matches('"').to_string())
        };
        field("O=").or_else(|| field("CN="))
    }
}

/// `ConvertTo-Json` gives an object for one package, an array for several, nothing for none.
pub fn parse_packages(json: &str) -> Vec<Package> {
    let json = json.trim();
    if json.is_empty() {
        return Vec::new();
    }
    serde_json::from_str::<Vec<Package>>(json)
        .or_else(|_| serde_json::from_str::<Package>(json).map(|p| vec![p]))
        .unwrap_or_default()
}

pub fn list() -> Vec<Package> {
    powershell(LIST_SCRIPT).map(|out| parse_packages(&out)).unwrap_or_default()
}

/// Removes a package for the current user.
pub fn remove(full_name: &str) -> Result<(), String> {
    // Package full names only use these characters; anything else is refused rather than quoted.
    if full_name.is_empty() || !full_name.chars().all(|c| c.is_ascii_alphanumeric() || "._~-".contains(c)) {
        return Err(format!("Not a package name: {full_name}"));
    }
    powershell(&format!("Remove-AppxPackage -Package '{full_name}' -ErrorAction Stop")).map(|_| ())
}

fn powershell(script: &str) -> Result<String, String> {
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: &str = r#"{"fullName":"SpotifyAB.SpotifyMusic_1.2.3.0_x64__zpdnekdrzrea0","family":"SpotifyAB.SpotifyMusic_zpdnekdrzrea0","name":"Spotify","version":"1.2.3.0","location":"C:\\Program Files\\WindowsApps\\SpotifyAB.SpotifyMusic_1.2.3.0_x64__zpdnekdrzrea0","publisher":"CN=453637B3-4E12-4CDF-B0D3-2A3C863BF6EF, O=Spotify AB, C=SE"}"#;

    #[test]
    fn parses_one_or_many_packages() {
        let one = parse_packages(ONE);
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].name, "Spotify");
        assert_eq!(one[0].publisher_name().as_deref(), Some("Spotify AB"));
        assert_eq!(parse_packages(&format!("[{ONE},{ONE}]")).len(), 2);
        assert!(parse_packages("").is_empty());
        assert!(parse_packages("garbage").is_empty());
    }

    #[test]
    fn falls_back_to_the_common_name() {
        let p = Package { publisher: Some("CN=Microsoft Corporation, O=Microsoft Corporation".into()), ..parse_packages(ONE)[0].clone() };
        assert_eq!(p.publisher_name().as_deref(), Some("Microsoft Corporation"));
        let p = Package { publisher: Some("CN=Some Dev".into()), ..p };
        assert_eq!(p.publisher_name().as_deref(), Some("Some Dev"));
    }

    #[test]
    fn refuses_odd_package_names() {
        assert!(remove("x'; Remove-Item C:\\ -Recurse; '").is_err());
        assert!(remove("").is_err());
    }
}
