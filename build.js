#!/usr/bin/env node

/**
 * Build script for minifying CSS, JS, and HTML files
 * Uses Terser for JS minification with proper string literal handling
 * Uses lightningcss for CSS minification
 * Uses html-minifier-terser for HTML minification
 */

import path from 'path'
import fs from 'fs-extra'
import { glob } from 'glob'
import { transform } from 'lightningcss'
import { minify } from 'terser'
import { minify as minifyHTML } from 'html-minifier-terser'
import { fileURLToPath } from 'url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const STATIC_DIR = path.join(__dirname, 'static')
const TEMPLATES_DIR = path.join(__dirname, 'templates')
const OUTPUT_DIR = STATIC_DIR

async function minifyJavaScript(filePath) {
	const outputPath = filePath.replace('.js', '.min.js')
	const fileName = path.basename(filePath)
	const minFileName = path.basename(outputPath)

	try {
		console.log(`Processing JS: ${fileName}`)

		const code = fs.readFileSync(filePath, 'utf8')

		const result = await minify(code, {
			compress: {
				drop_console: false,
				passes: 2,
				unsafe: false
			},
			mangle: true,
			format: {
				comments: false,
				beautify: false,
				ascii_only: true,
				semicolons: true
			}
		})

		if (!result.code) {
			throw new Error('Minification produced no output')
		}

		await fs.writeFile(outputPath, result.code)
		console.log(`✓ Minified JS: ${fileName} → ${minFileName}`)

		const originalSize = fs.statSync(filePath).size
		const minifiedSize = fs.statSync(outputPath).size
		const ratio = ((minifiedSize / originalSize) * 100).toFixed(2)
		console.log(
			`  Original: ${(originalSize / 1024).toFixed(2)}KB, Minified: ${(minifiedSize / 1024).toFixed(2)}KB (${ratio}% of original)`
		)

		const lineCount = result.code.split('\n').length
		console.log(`  Line count: ${lineCount}`)
	} catch (error) {
		console.error(`✗ Error minifying ${fileName}:`, error)
	}
}

function minifyCSS(filePath) {
	const outputPath = filePath.replace('.css', '.min.css')
	const fileName = path.basename(filePath)
	const minFileName = path.basename(outputPath)

	try {
		const source = fs.readFileSync(filePath)
		const result = transform({
			filename: filePath,
			code: source,
			minify: true,
			sourceMap: false
		})

		fs.writeFileSync(outputPath, result.code)
		console.log(`✓ Minified CSS: ${fileName} → ${minFileName}`)

		const originalSize = fs.statSync(filePath).size
		const minifiedSize = fs.statSync(outputPath).size
		const ratio = ((minifiedSize / originalSize) * 100).toFixed(2)
		console.log(
			`  Original: ${(originalSize / 1024).toFixed(2)}KB, Minified: ${(minifiedSize / 1024).toFixed(2)}KB (${ratio}% of original)`
		)
	} catch (error) {
		console.error(`✗ Error minifying ${fileName}:`, error)
	}
}

async function minifyHTMLFile(filePath) {
	const outputPath = filePath.replace('.html', '.min.html')
	const fileName = path.basename(filePath)
	const minFileName = path.basename(outputPath)

	try {
		console.log(`Processing HTML: ${fileName}`)

		const html = fs.readFileSync(filePath, 'utf8')

		const preserveRegex =
			/(<code[\s\S]*?<\/code>|<pre[\s\S]*?<\/pre>|<div\s+class=["']ascii-art["'][\s\S]*?<\/div>)/gi
		const preservedBlocks = []
		let preprocessedHtml = html.replace(preserveRegex, (match) => {
			if (match.startsWith('<code') || match.startsWith('<pre')) {
				const openTagMatch = match.match(/^<(code|pre)([^>]*)>/)
				const closeTagMatch = match.match(/<\/(code|pre)>$/)

				if (openTagMatch && closeTagMatch) {
					const tagName = openTagMatch[1]
					const attributes = openTagMatch[2]
					const openTag = `<${tagName}${attributes}>`
					const closeTag = `</${tagName}>`

					const content = match.slice(openTag.length, match.length - closeTag.length)
					const encodedContent = content.replace(/\n/g, '&#10;')

					const encodedBlock = `${openTag}${encodedContent}${closeTag}`
					const placeholder = `__PRESERVED_BLOCK_${preservedBlocks.length}__`
					preservedBlocks.push(encodedBlock)
					return placeholder
				}
			}

			if (
				(match.startsWith('<div') && match.includes('class="ascii-art"')) ||
				match.includes("class='ascii-art'")
			) {
				const openTagMatch = match.match(/^<(div)([^>]*)>/)
				const closeTagMatch = match.match(/<\/(div)>$/)

				if (openTagMatch && closeTagMatch) {
					const tagName = openTagMatch[1]
					const attributes = openTagMatch[2]
					const openTag = `<${tagName}${attributes}>`
					const closeTag = `</${tagName}>`

					const content = match.slice(openTag.length, match.length - closeTag.length)
					const encodedContent = content.replace(/\n/g, '&#10;')

					const encodedBlock = `${openTag}${encodedContent}${closeTag}`
					const placeholder = `__PRESERVED_BLOCK_${preservedBlocks.length}__`
					preservedBlocks.push(encodedBlock)
					return placeholder
				}
			}

			const placeholder = `__PRESERVED_BLOCK_${preservedBlocks.length}__`
			preservedBlocks.push(match)
			return placeholder
		})

		const minifiedHtml = await minifyHTML(preprocessedHtml, {
			collapseWhitespace: true,
			conservativeCollapse: false,
			preserveLineBreaks: false,
			minifyCSS: true,
			minifyJS: true,
			removeComments: true,
			removeAttributeQuotes: false,
			removeRedundantAttributes: true,
			removeScriptTypeAttributes: true,
			removeStyleLinkTypeAttributes: true,
			useShortDoctype: true,
			quoteCharacter: '"'
		})

		let singleLineHtml = minifiedHtml.replace(/\r?\n|\r/g, '')

		let finalHtml = singleLineHtml
		preservedBlocks.forEach((block, index) => {
			finalHtml = finalHtml.replace(`__PRESERVED_BLOCK_${index}__`, block)
		})

		await fs.writeFile(outputPath, finalHtml)
		console.log(`✓ Minified HTML: ${fileName} → ${minFileName}`)

		const originalSize = fs.statSync(filePath).size
		const minifiedSize = fs.statSync(outputPath).size
		const ratio = ((minifiedSize / originalSize) * 100).toFixed(2)
		console.log(
			`  Original: ${(originalSize / 1024).toFixed(2)}KB, Minified: ${(minifiedSize / 1024).toFixed(2)}KB (${ratio}% of original)`
		)
	} catch (error) {
		console.error(`✗ Error minifying ${fileName}:`, error)
	}
}

async function build() {
	console.log('🚀 Starting build process...')

	await fs.ensureDir(OUTPUT_DIR)

	const jsFiles = glob.sync(`${STATIC_DIR}/**/*.js`, { ignore: [`${STATIC_DIR}/**/*.min.js`] })
	const cssFiles = glob.sync(`${STATIC_DIR}/**/*.css`, { ignore: [`${STATIC_DIR}/**/*.min.css`] })
	const htmlFiles = glob.sync(`${TEMPLATES_DIR}/**/*.html`, {
		ignore: [`${TEMPLATES_DIR}/**/*.min.html`]
	})

	console.log(
		`Found ${jsFiles.length} JavaScript files, ${cssFiles.length} CSS files, and ${htmlFiles.length} HTML files to minify.`
	)

	const jsPromises = jsFiles.map(minifyJavaScript)
	await Promise.all(jsPromises)

	cssFiles.forEach(minifyCSS)

	const htmlPromises = htmlFiles.map(minifyHTMLFile)
	await Promise.all(htmlPromises)

	console.log('✅ Build completed successfully!')
}

build().catch((error) => {
	console.error('❌ Build failed:', error)
	process.exit(1)
})
