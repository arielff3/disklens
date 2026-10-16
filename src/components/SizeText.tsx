import { splitBytes } from "../format";

export function SizeText({ bytes }: { bytes: number }) {
  const { value, unit } = splitBytes(bytes);
  return (
    <div className="size">
      {value}
      <span className="unit">{unit}</span>
    </div>
  );
}
