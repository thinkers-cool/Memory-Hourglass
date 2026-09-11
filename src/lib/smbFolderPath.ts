export function relativeFolderPath(rootPath: string, absolutePath: string): string {
  const normalizedRoot = rootPath.replace(/\/+$/, "");
  const normalizedAbsolute = absolutePath.replace(/\/+$/, "");
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
  const normalizedRoot = rootPath.replace(/\/+$/, "");
  const trimmed = relativePath.replace(/^\/+/, "").replace(/\/+$/, "");
  if (!trimmed) {
    return normalizedRoot;
  }
  return `${normalizedRoot}/${trimmed}`;
}
