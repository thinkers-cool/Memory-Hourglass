export function optionRowClass(active: boolean) {
  return active
    ? "bg-interactive-selected font-medium text-primary"
    : "transition-colors hover:bg-interactive-hover-strong";
}

export function menuPickerItemClass(active: boolean) {
  return active
    ? "menu-picker-item-active font-medium text-primary"
    : "transition-colors hover:bg-interactive-hover-strong";
}

export function listRowClass(active?: boolean) {
  return `border border-transparent transition-colors hover:bg-interactive-hover hover:border-divider-subtle ${
    active ? "bg-interactive-selected border-interactive-selected-border" : ""
  }`;
}

export function listRowTitleClass(active?: boolean) {
  return active ? "font-medium text-primary" : "text-content-secondary";
}

export function listRowBadgeClass(active?: boolean) {
  return `transition-colors ${
    active
      ? "bg-interactive-selected-strong text-primary"
      : "bg-badge-idle text-content-muted group-hover:bg-badge-hover"
  }`;
}
