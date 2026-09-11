export function parseSmbHost(raw: string): string | null {
  const trimmed = raw
    .trim()
    .replace(/^smb:\/\//i, "")
    .replace(/\/+$/, "");
  if (!trimmed) return null;
  const slash = trimmed.indexOf("/");
  const host = (slash >= 0 ? trimmed.slice(0, slash) : trimmed).trim();
  if (!host || /\s/.test(host)) return null;
  return host;
}

export function parseSmbAddress(
  raw: string,
): { host: string; share: string } | null {
  const trimmed = raw.trim().replace(/^smb:\/\//i, "");
  const slash = trimmed.indexOf("/");
  if (slash <= 0) return null;
  const host = trimmed.slice(0, slash).trim();
  const share = trimmed
    .slice(slash + 1)
    .replace(/^\/+/, "")
    .split("/")[0]
    ?.trim();
  if (!host || !share) return null;
  return { host, share };
}

export function formatSmbAddress(host: string, share: string): string {
  return `smb://${host}/${share}`;
}
