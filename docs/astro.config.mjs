// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
	site: 'https://winxmerge-site.pages.dev',
	integrations: [
		starlight({
			title: 'WinXMerge',
			logo: {
				src: './src/assets/app-icon.svg',
			},
			social: [
				{ icon: 'github', label: 'GitHub', href: 'https://github.com/masak1yu/winxmerge' },
			],
			defaultLocale: 'en',
			locales: {
				en: { label: 'English', lang: 'en' },
				ja: { label: '日本語', lang: 'ja' },
			},
			sidebar: [
				{
					label: 'Getting Started',
					translations: { ja: 'はじめに' },
					items: [
						{ slug: 'guides/introduction' },
						{ slug: 'guides/installation' },
						{ slug: 'guides/quickstart' },
					],
				},
				{
					label: 'Features',
					translations: { ja: '機能' },
					items: [
						{ slug: 'features/file-comparison' },
						{ slug: 'features/three-way-merge' },
						{ slug: 'features/folder-comparison' },
						{ slug: 'features/csv-excel' },
						{ slug: 'features/image-comparison' },
						{ slug: 'features/syntax-highlighting' },
						{ slug: 'features/inline-editing' },
						{ slug: 'features/search-replace' },
						{ slug: 'features/filters' },
						{ slug: 'features/export' },
					],
				},
				{
					label: 'Integrations',
					translations: { ja: '外部連携' },
					items: [
						{ slug: 'integrations/git' },
						{ slug: 'integrations/macos-finder' },
						{ slug: 'integrations/plugins' },
					],
				},
				{
					label: 'Reference',
					translations: { ja: 'リファレンス' },
					items: [
						{ slug: 'reference/keyboard-shortcuts' },
						{ slug: 'reference/settings' },
						{ slug: 'reference/tech-stack' },
					],
				},
			],
		}),
	],
});
