export function moveItem<T>(items: T[], sourceIndex: number, targetIndex: number): boolean {
  if (
    sourceIndex < 0 ||
    sourceIndex >= items.length ||
    targetIndex < 0 ||
    targetIndex >= items.length ||
    sourceIndex === targetIndex
  ) {
    return false;
  }

  const [item] = items.splice(sourceIndex, 1);
  if (item === undefined) return false;
  items.splice(targetIndex, 0, item);
  return true;
}
