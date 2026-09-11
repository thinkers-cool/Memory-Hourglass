import { LocaleSwitcher, type LocaleMenuPlacement } from "./LocaleSwitcher";
import { ThemeSwitcher } from "./ThemeSwitcher";

export function AppearanceControls({
  menuPlacement = "float-bottom-end",
  size = "sm",
  layout = "vertical",
}: {
  menuPlacement?: LocaleMenuPlacement;
  size?: "sm" | "nav";
  layout?: "vertical" | "horizontal";
}) {
  return (
    <div
      className={
        layout === "horizontal"
          ? "flex flex-row items-center gap-1.5"
          : "flex flex-col items-center gap-1.5"
      }
    >
      <LocaleSwitcher menuPlacement={menuPlacement} size={size} />
      <ThemeSwitcher menuPlacement={menuPlacement} size={size} />
    </div>
  );
}
