import i18n from "../i18n";

export const RATING_LEVELS = [1, 2, 3, 4, 5] as const;
export const RATING_STAR_FILLED = "★";
export const RATING_STAR_EMPTY = "☆";

export function isRatingStarFilled(
  value: number | null | undefined,
  star: number,
): boolean {
  return (value ?? 0) >= star;
}

export function formatRatingStars(rating: number): string {
  if (rating <= 0) {
    return "";
  }
  const clamped = Math.min(5, Math.max(1, Math.round(rating)));
  return RATING_STAR_FILLED.repeat(clamped);
}

export function ratingStarAriaLabel(rating: number): string {
  return i18n.t("common:rating.aria", { count: rating });
}

export function formatRatingMinLabel(minRating: number): string {
  const stars = formatRatingStars(minRating);
  return minRating >= 5 ? stars : `${stars}+`;
}
