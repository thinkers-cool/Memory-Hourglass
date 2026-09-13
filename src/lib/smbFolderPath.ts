function normalizeFolderPath(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+$/, "");
}

export function relativeFolderPath(
  rootPath: string,
  absolutePath: string,
): string {
  const normalizedRoot = normalizeFolderPath(rootPath);
  const normalizedAbsolute = normalizeFolderPath(absolutePath);
  if (normalizedAbsolute === normalizedRoot) {
    return "";
  }
  const prefix = `${normalizedRoot}/`;
  if (!normalizedAbsolute.startsWith(prefix)) {
    return "";
  }
  return normalizedAbsolute.slice(prefix.length);
}

export function joinFolderPath(rootPath: string, relativePath: string): string {
  const normalizedRoot = normalizeFolderPath(rootPath);
  const trimmed = relativePath
    .replace(/\\/g, "/")
    .replace(/^\/+/, "")
    .replace(/\/+$/, "");
  if (!trimmed) {
    return normalizedRoot;
  }
  return `${normalizedRoot}/${trimmed}`;
}
