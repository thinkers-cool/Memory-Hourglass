import { ghostBtnClass } from "../lib/buttonClass";
import {
  isRatingStarFilled,
  RATING_LEVELS,
  RATING_STAR_EMPTY,
  RATING_STAR_FILLED,
  ratingStarAriaLabel,
} from "../lib/ratingStars";

export function RatingRow({
  value,
  size = "xs",
  compact = false,
  onSelect,
}: {
  value?: number | null;
  size?: "xs" | "sm";
  compact?: boolean;
  onSelect: (rating: number) => void;
}) {
  return (
    <div className="join">
      {RATING_LEVELS.map((rating) => {
        const filled = isRatingStarFilled(value, rating);
        const star = filled ? RATING_STAR_FILLED : RATING_STAR_EMPTY;

        return (
          <button
            key={rating}
            type="button"
            aria-label={ratingStarAriaLabel(rating)}
            aria-pressed={filled}
            title={ratingStarAriaLabel(rating)}
            className={
              compact
                ? `join-item ${
                    filled
                      ? "btn btn-primary btn-xs min-h-0 h-7 w-7 px-0 text-sm font-semibold leading-none text-amber-100"
                      : `${ghostBtnClass("btn-xs min-h-0 h-7 w-7 px-0 text-sm font-semibold leading-none text-amber-300/55")} border border-control-border`
                  }`
                : `join-item btn btn-${size} min-h-0 h-8 w-8 px-0 text-base font-semibold leading-none ${
                    filled
                      ? "btn-primary text-amber-100"
                      : "btn-outline text-amber-500/55"
                  }`
            }
            onClick={() => onSelect(rating)}
          >
            {star}
          </button>
        );
      })}
    </div>
  );
}
