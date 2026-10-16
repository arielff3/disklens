// Bump VERSION with each release; the download links follow it.
export const VERSION = "0.1.0";
export const REPO = "https://github.com/arielff3/disklens";
export const DOWNLOAD = `${REPO}/releases/download/v${VERSION}/Disklens_${VERSION}_x64-setup.exe`;
export const RELEASES = `${REPO}/releases`;
// Vercel sets this to the production domain; it makes social preview images absolute.
const host = process.env.VERCEL_PROJECT_PRODUCTION_URL;
export const SITE_URL = host ? `https://${host}` : "http://localhost:3000";
