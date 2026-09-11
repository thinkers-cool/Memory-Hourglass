import type { AppTheme } from "./theme";

export type ThemeColorPair = {
  label: string;
  foreground: string;
  background: string;
  minRatio: number;
};

export const THEME_CONTRAST_PAIRS: Record<AppTheme, ThemeColorPair[]> = {
  vault: [
    {
      label: "base-content on base-300",
      foreground: "oklch(92% 0.005 264)",
      background: "oklch(9.2% 0.004 264)",
      minRatio: 4.5,
    },
    {
      label: "primary-content on primary",
      foreground: "oklch(16% 0.03 88)",
      background: "oklch(84% 0.13 88)",
      minRatio: 4.5,
    },
    {
      label: "content-muted on base-200",
      foreground: "oklch(72% 0.015 264)",
      background: "oklch(11.5% 0.004 264)",
      minRatio: 4.5,
    },
  ],
  neon: [
    {
      label: "base-content on base-300",
      foreground: "oklch(92% 0.01 280)",
      background: "oklch(15% 0.07 275)",
      minRatio: 4.5,
    },
    {
      label: "primary-content on primary",
      foreground: "oklch(98% 0.01 285)",
      background: "oklch(50% 0.21 285)",
      minRatio: 4.5,
    },
    {
      label: "secondary-content on secondary",
      foreground: "oklch(16% 0.03 185)",
      background: "oklch(72% 0.14 185)",
      minRatio: 4.5,
    },
    {
      label: "content-muted on base-200",
      foreground: "oklch(72% 0.02 280)",
      background: "oklch(22% 0.09 278)",
      minRatio: 4.5,
    },
  ],
  pulse: [
    {
      label: "base-content on base-300",
      foreground: "oklch(92% 0.005 265)",
      background: "oklch(12% 0.04 265)",
      minRatio: 4.5,
    },
    {
      label: "primary-content on primary",
      foreground: "oklch(98% 0.01 295)",
      background: "oklch(53% 0.23 295)",
      minRatio: 4.5,
    },
    {
      label: "secondary-content on secondary",
      foreground: "oklch(98% 0.01 264)",
      background: "oklch(50% 0.19 264)",
      minRatio: 4.5,
    },
    {
      label: "content-muted on base-200",
      foreground: "oklch(72% 0.02 265)",
      background: "oklch(18% 0.05 268)",
      minRatio: 4.5,
    },
  ],
  atlas: [
    {
      label: "base-content on base-300",
      foreground: "oklch(28% 0.04 55)",
      background: "oklch(91% 0.016 82)",
      minRatio: 4.5,
    },
    {
      label: "primary-content on primary",
      foreground: "oklch(98% 0.01 85)",
      background: "oklch(38% 0.08 55)",
      minRatio: 4.5,
    },
    {
      label: "content-muted on base-200",
      foreground: "oklch(48% 0.03 55)",
      background: "oklch(95.5% 0.013 85)",
      minRatio: 4.5,
    },
  ],
  slate: [
    {
      label: "base-content on base-300",
      foreground: "oklch(92% 0.01 265)",
      background: "oklch(16% 0.02 265)",
      minRatio: 4.5,
    },
    {
      label: "primary-content on primary",
      foreground: "oklch(98% 0.01 275)",
      background: "oklch(50% 0.18 275)",
      minRatio: 4.5,
    },
    {
      label: "content-muted on base-200",
      foreground: "oklch(68% 0.02 265)",
      background: "oklch(20% 0.025 265)",
      minRatio: 4.5,
    },
  ],
  sakura: [
    {
      label: "base-content on base-300",
      foreground: "oklch(32% 0.06 350)",
      background: "oklch(93% 0.025 350)",
      minRatio: 4.5,
    },
    {
      label: "primary-content on primary",
      foreground: "oklch(98% 0.01 350)",
      background: "oklch(50% 0.18 350)",
      minRatio: 4.5,
    },
    {
      label: "content-muted on base-200",
      foreground: "oklch(52% 0.05 350)",
      background: "oklch(96.5% 0.02 350)",
      minRatio: 4.5,
    },
  ],
  moss: [
    {
      label: "base-content on base-300",
      foreground: "oklch(90% 0.02 145)",
      background: "oklch(14% 0.04 145)",
      minRatio: 4.5,
    },
    {
      label: "primary-content on primary",
      foreground: "oklch(16% 0.04 145)",
      background: "oklch(72% 0.16 145)",
      minRatio: 4.5,
    },
    {
      label: "content-muted on base-200",
      foreground: "oklch(68% 0.03 145)",
      background: "oklch(18% 0.05 145)",
      minRatio: 4.5,
    },
  ],
  solar: [
    {
      label: "base-content on base-300",
      foreground: "oklch(30% 0.05 55)",
      background: "oklch(90% 0.038 72)",
      minRatio: 4.5,
    },
    {
      label: "primary-content on primary",
      foreground: "oklch(18% 0.04 65)",
      background: "oklch(68% 0.16 65)",
      minRatio: 4.5,
    },
    {
      label: "content-muted on base-200",
      foreground: "oklch(50% 0.04 60)",
      background: "oklch(94.5% 0.032 75)",
      minRatio: 4.5,
    },
  ],
  ink: [
    {
      label: "base-content on base-300",
      foreground: "oklch(94% 0 0)",
      background: "oklch(10% 0 0)",
      minRatio: 4.5,
    },
    {
      label: "primary-content on primary",
      foreground: "oklch(10% 0 0)",
      background: "oklch(94% 0 0)",
      minRatio: 4.5,
    },
    {
      label: "content-muted on base-200",
      foreground: "oklch(68% 0 0)",
      background: "oklch(14% 0 0)",
      minRatio: 4.5,
    },
  ],
  coral: [
    {
      label: "base-content on base-300",
      foreground: "oklch(28% 0.05 220)",
      background: "oklch(91% 0.024 195)",
      minRatio: 4.5,
    },
    {
      label: "primary-content on primary",
      foreground: "oklch(98% 0.01 25)",
      background: "oklch(52% 0.2 25)",
      minRatio: 4.5,
    },
    {
      label: "content-muted on base-200",
      foreground: "oklch(48% 0.04 210)",
      background: "oklch(95.5% 0.02 200)",
      minRatio: 4.5,
    },
  ],
  oxide: [
    {
      label: "base-content on base-300",
      foreground: "oklch(90% 0.02 70)",
      background: "oklch(12% 0.02 50)",
      minRatio: 4.5,
    },
    {
      label: "primary-content on primary",
      foreground: "oklch(14% 0.03 55)",
      background: "oklch(65% 0.14 55)",
      minRatio: 4.5,
    },
    {
      label: "content-muted on base-200",
      foreground: "oklch(68% 0.03 60)",
      background: "oklch(16% 0.025 48)",
      minRatio: 4.5,
    },
  ],
  arctic: [
    {
      label: "base-content on base-300",
      foreground: "oklch(28% 0.05 250)",
      background: "oklch(91% 0.024 235)",
      minRatio: 4.5,
    },
    {
      label: "primary-content on primary",
      foreground: "oklch(98% 0.01 235)",
      background: "oklch(52% 0.14 235)",
      minRatio: 4.5,
    },
    {
      label: "content-muted on base-200",
      foreground: "oklch(48% 0.04 240)",
      background: "oklch(95.5% 0.02 230)",
      minRatio: 4.5,
    },
  ],
  prism: [
    {
      label: "base-content on base-300",
      foreground: "oklch(92% 0.01 285)",
      background: "oklch(13% 0.03 285)",
      minRatio: 4.5,
    },
    {
      label: "primary-content on primary",
      foreground: "oklch(98% 0.01 295)",
      background: "oklch(58% 0.24 295)",
      minRatio: 4.5,
    },
    {
      label: "content-muted on base-200",
      foreground: "oklch(70% 0.02 285)",
      background: "oklch(17% 0.04 285)",
      minRatio: 4.5,
    },
  ],
};
