export function ShortcutHint({ children }: { children: string }) {
  return (
    <span className="ml-1 text-[10px] font-normal uppercase tracking-wide text-content-faint">
      {children}
    </span>
  );
}
