export function loadStoredNumber(
  key: string,
  fallback: number,
  clamp: (value: number) => number,
): number {
  try {
    const raw = localStorage.getItem(key);
    if (raw === null) {
      return fallback;
    }
    const parsed = Number.parseInt(raw, 10);
    if (!Number.isFinite(parsed)) {
      return fallback;
    }
    return clamp(parsed);
  } catch {
    return fallback;
  }
}

export function saveStoredNumber(key: string, value: number): void {
  try {
    localStorage.setItem(key, String(value));
  } catch {
    return;
  }
}

export function loadStoredJson<T>(
  key: string,
  fallback: T,
  validate: (value: unknown) => T | null,
): T {
  try {
    const raw = localStorage.getItem(key);
    if (raw === null) {
      return fallback;
    }
    const parsed: unknown = JSON.parse(raw);
    const validated = validate(parsed);
    return validated ?? fallback;
  } catch {
    return fallback;
  }
}

export function saveStoredJson(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    return;
  }
}
