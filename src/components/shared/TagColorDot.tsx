import { tagColor } from "../../lib/libraryIndicators";

export function TagColorDot({
  color,
  className = "h-2.5 w-2.5",
}: {
  color: string | null | undefined;
  className?: string;
}) {
  return (
    <span
      className={`inline-block shrink-0 rounded-full border border-divider-subtle ${className}`}
      style={{ backgroundColor: tagColor(color) }}
    />
  );
}
