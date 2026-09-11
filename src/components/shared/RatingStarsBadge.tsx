import { formatRatingStars } from "../../lib/ratingStars";

export function RatingStarsBadge({ rating }: { rating: number }) {
  const stars = formatRatingStars(rating);
  if (!stars) {
    return null;
  }

  return (
    <span
      className="inline-flex items-center rounded-md border border-white/15 bg-neutral-950/75 px-1.5 py-0.5 text-[10px] font-semibold leading-none tracking-tight text-amber-300 shadow-sm backdrop-blur-md"
      aria-label={`Rating ${rating}`}
    >
      <span aria-hidden>{stars}</span>
    </span>
  );
}
