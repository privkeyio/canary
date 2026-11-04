import fs from 'node:fs'
import path from 'node:path'
import sitemap from './sitemap'

function robotsDisallowedPaths(): string[] {
  const robots = fs.readFileSync(path.join(process.cwd(), 'public', 'robots.txt'), 'utf8')
  return robots
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.toLowerCase().startsWith('disallow:'))
    .map((line) => line.slice('disallow:'.length).trim())
    .filter(Boolean)
}

describe('sitemap', () => {
  const entries = sitemap()
  const paths = entries.map((entry) => new URL(entry.url).pathname)

  it('lists the public marketing pages on the canonical origin', () => {
    expect(paths).toEqual(['/', '/cloud', '/private', '/contact', '/donations'])
    for (const entry of entries) {
      const url = new URL(entry.url)
      expect(url.origin).toBe('https://canarybitcoin.com')
      expect(url.hash).toBe('')
    }
  })

  it('never lists routes disallowed in robots.txt', () => {
    const disallowed = robotsDisallowedPaths()
    expect(disallowed).toEqual(expect.arrayContaining(['/api/', '/wallets/', '/settings/', '/sign-in', '/sign-up', '/forgot-password']))
    for (const pathname of paths) {
      for (const prefix of disallowed) {
        expect(pathname.startsWith(prefix)).toBe(false)
      }
    }
  })

  it('is served by the app route instead of a static public file', () => {
    // A static public/sitemap.xml conflicts with app/sitemap.ts and depends on
    // host checkout file permissions inside the Docker image.
    expect(fs.existsSync(path.join(process.cwd(), 'public', 'sitemap.xml'))).toBe(false)
  })
})

describe('sitemap canonicals', () => {
  // A sitemap URL whose canonical points elsewhere tells Google not to index it.
  const metadataModules: Record<string, () => Promise<{ metadata?: { alternates?: { canonical?: unknown } } }>> = {
    '/cloud': () => import('./cloud/page'),
    '/private': () => import('./private/page'),
    '/contact': () => import('./contact/layout'),
    '/donations': () => import('./donations/layout'),
  }

  it.each(Object.entries(metadataModules))('%s declares itself as canonical', async (pagePath, load) => {
    const { metadata } = await load()
    expect(metadata?.alternates?.canonical).toBe(`https://canarybitcoin.com${pagePath}`)
  })
})
