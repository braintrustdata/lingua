export function centerText(
  text: string,
  width: number,
  padChar: string = " "
): string {
  const textLength = text.length;
  if (textLength >= width) return text;

  const totalPadding = width - textLength;
  const leftPadding = Math.floor(totalPadding / 2);
  const rightPadding = totalPadding - leftPadding;

  return padChar.repeat(leftPadding) + text + padChar.repeat(rightPadding);
}
