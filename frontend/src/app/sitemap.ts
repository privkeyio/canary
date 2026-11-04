import type { MetadataRoute } from 'next'

const SITE_URL = 'https://canarybitcoin.com'

// Public marketing pages only. Locale is negotiated per visitor via cookie on
// the same URL, so there are no per-locale URLs to advertise as hreflang
// alternates. Never list routes disallowed in public/robots.txt.
export const PUBLIC_SITEMAP_PATHS = ['/', '/cloud', '/private', '/contact', '/donations'] as const

export default function sitemap(): MetadataRoute.Sitemap {
  return PUBLIC_SITEMAP_PATHS.map((path) => ({
    url: path === '/' ? `${SITE_URL}/` : `${SITE_URL}${path}`,
    changeFrequency: path === '/' ? 'weekly' : 'monthly',
    priority: path === '/' ? 1 : 0.8,
  }))
}
