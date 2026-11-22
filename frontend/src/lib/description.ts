import { MODDB_URL } from './constants.ts';

/**
 * ModDB descriptions and changelogs are HTML written in the ModDB's editor. react-markdown would
 * treat text after a blank line as Markdown (indented lines become code blocks), so the HTML is
 * kept in a single HTML block: blank lines are dropped and the whole is wrapped in a `<div>`.
 * The result is still sanitized when rendered.
 */
export function prepareDescription(html: string): string {
  const body = html
    .replace(/\r\n?/g, '\n')
    .split('\n')
    .filter((line) => line.trim() !== '')
    .join('\n');

  return `<div>\n${body}\n</div>`;
}

/** Links and images in descriptions may point at the ModDB itself; anything but http(s) and mail is dropped. */
export function absoluteUrl(url: string | null | undefined): string | undefined {
  if (!url) return undefined;
  const trimmed = url.trim();
  if (/^https?:\/\//i.test(trimmed) || /^mailto:/i.test(trimmed)) return trimmed;
  if (trimmed.startsWith('//')) return `https:${trimmed}`;
  if (trimmed.startsWith('/')) return `${MODDB_URL}${trimmed}`;

  return undefined;
}
