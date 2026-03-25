import { themes as prismThemes } from 'prism-react-renderer';
import type { Config } from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'ROSQL',
  tagline: 'The open source query language that natively speaks robot',
  favicon: 'img/favicon.ico',

  future: {
    v4: true,
  },

  url: 'https://rosql.org',
  baseUrl: '/',

  organizationName: 'RobotOpsInc',
  projectName: 'rosql',

  onBrokenLinks: 'throw',
  markdown: {
    hooks: {
      onBrokenMarkdownLinks: 'warn',
    },
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          editUrl: 'https://github.com/RobotOpsInc/rosql/tree/main/website/',
          includeCurrentVersion: false, // hide "Next" — edit versioned_docs/version-0.1/ for patches
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
        sitemap: {
          changefreq: 'weekly',
          priority: 0.5,
        },
      } satisfies Preset.Options,
    ],
  ],

  plugins: [
    // PostHog analytics — filter by $host = 'rosql.org' in dashboard
    // (same project as robotops.com, EU server)
    [
      'posthog-docusaurus',
      {
        apiKey: 'phc_gr24NqI4C1x3d60MNFYHlWv1KQARcUa15WXFE8VTrE4',
        appUrl: 'https://eu.i.posthog.com',
        enableInDevelopment: false,
      },
    ],
    // Enable WebAssembly bundling for the ROSQL REPL
    function wasmPlugin() {
      return {
        name: 'wasm-plugin',
        configureWebpack() {
          return {
            experiments: {
              asyncWebAssembly: true,
            },
          };
        },
      };
    },
  ],

  themeConfig: {
    image: 'img/og-image.png',
    metadata: [
      { name: 'keywords', content: 'rosql, ros2, robotics, query language, opentelemetry, telemetry, robot observability' },
      { name: 'twitter:card', content: 'summary_large_image' },
    ],
    colorMode: {
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'ROSQL',
      logo: {
        alt: 'ROSQL logo',
        src: 'img/logo.svg',
      },
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'docs',
          position: 'left',
          label: 'Docs',
        },
        { to: '/examples', label: 'Examples', position: 'left' },
        { to: '/playground', label: 'Try It', position: 'left' },
        { to: '/faq', label: 'FAQ', position: 'left' },
        { to: '/benchmarks', label: 'Benchmarks', position: 'left' },
        { to: '/contributing', label: 'Contributing', position: 'left' },
        {
          type: 'docsVersionDropdown',
          position: 'right',
        },
        {
          href: 'https://github.com/RobotOpsInc/rosql',
          position: 'right',
          className: 'header-github-link',
          'aria-label': 'GitHub repository',
        },
        {
          href: 'https://robotops.com',
          label: 'Robot Ops Platform →',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            { label: 'Quickstart', to: '/docs/quickstart' },
            { label: 'CLI Reference', to: '/docs/cli' },
            { label: 'WASM / Browser', to: '/docs/wasm' },
            { label: 'Schema Reference', to: '/docs/schema-reference' },
          ],
        },
        {
          title: 'Community',
          items: [
            { label: 'GitHub', href: 'https://github.com/RobotOpsInc/rosql' },
            { label: 'Issues', href: 'https://github.com/RobotOpsInc/rosql/issues' },
            { label: 'Contributing', to: '/contributing' },
            { label: 'Contact', href: 'mailto:devs@robotops.com' },
          ],
        },
        {
          title: 'More',
          items: [
            { label: 'crates.io', href: 'https://crates.io/crates/rosql' },
            { label: 'npm', href: 'https://www.npmjs.com/package/@robotops/rosql' },
            { label: 'Robot Ops Platform', href: 'https://robotops.com' },
            { label: 'FAQ', to: '/faq' },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Robot Ops, Inc. Licensed under Apache 2.0.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ['bash', 'json', 'toml', 'rust', 'sql'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
