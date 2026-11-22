import { Typography } from '@mantine/core';
import Markdown from 'react-markdown';
import rehypeRaw from 'rehype-raw';
import rehypeSanitize, { defaultSchema } from 'rehype-sanitize';
import Anchor from '@/elements/typography/Anchor.tsx';
import { absoluteUrl, prepareDescription } from '../lib/description.ts';

// Keep the layout attributes ModDB descriptions use; inline styles (colours meant for the ModDB's
// light page) and scripts are removed.
const sanitizeSchema = {
  ...defaultSchema,
  tagNames: [...(defaultSchema.tagNames ?? []), 'center'],
  attributes: {
    ...defaultSchema.attributes,
    '*': [...(defaultSchema.attributes?.['*'] ?? []), 'align', 'width', 'height'],
  },
};

/** Renders the HTML of a ModDB description or changelog, sanitized. */
export default function ModDescription({ html }: { html: string }) {
  return (
    <Typography>
      <Markdown
        rehypePlugins={[rehypeRaw, [rehypeSanitize, sanitizeSchema]]}
        components={{
          a: ({ href, children }) => (
            <Anchor href={absoluteUrl(href)} target='_blank' rel='noopener noreferrer'>
              {children}
            </Anchor>
          ),
          img: ({ src, alt, width, height }) => (
            <img
              src={absoluteUrl(typeof src === 'string' ? src : undefined)}
              alt={alt ?? ''}
              width={width}
              height={height}
              loading='lazy'
              referrerPolicy='no-referrer'
              style={{ maxWidth: '100%', height: 'auto', display: 'inline-block' }}
            />
          ),
          // The panel's CSS reset strips list markers; descriptions rely on them.
          ul: ({ children }) => <ul style={{ listStyleType: 'disc', paddingLeft: '1.5rem' }}>{children}</ul>,
          ol: ({ children }) => <ol style={{ listStyleType: 'decimal', paddingLeft: '1.5rem' }}>{children}</ol>,
        }}
      >
        {prepareDescription(html)}
      </Markdown>
    </Typography>
  );
}
