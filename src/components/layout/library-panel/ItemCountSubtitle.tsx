import { useTranslation } from "react-i18next";

export function ItemCountSubtitle({ count }: { count: number }) {
  const { t } = useTranslation("library");
  return <span className="tabular-nums">{t("panel.items", { count })}</span>;
}
