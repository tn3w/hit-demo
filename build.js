#!/usr/bin/env node

/**
 * Build script for minifying CSS and JS files
 * Uses Terser for JS minification with proper string literal handling
 * Uses lightningcss for CSS minification
 */

import path from 'path';
import fs from 'fs-extra';
import { glob } from 'glob';
import { transform } from 'lightningcss';
import { minify } from 'terser';
import { fileURLToPath } from 'url';

// Get __dirname equivalent in ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Directories
const STATIC_DIR = path.join(__dirname, 'static');
const OUTPUT_DIR = STATIC_DIR;

async function minifyJavaScript(filePath) {
  const outputPath = filePath.replace('.js', '.min.js');
  const fileName = path.basename(filePath);
  const minFileName = path.basename(outputPath);
  
  try {
    console.log(`Processing JS: ${fileName}`);
    
    // Read the file content
    const code = fs.readFileSync(filePath, 'utf8');
    
    // Use Terser for minification with proper string literal handling
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
        ascii_only: true, // Escape Unicode characters
        semicolons: true
      }
    });
    
    if (!result.code) {
      throw new Error('Minification produced no output');
    }
    
    await fs.writeFile(outputPath, result.code);
    console.log(`✓ Minified JS: ${fileName} → ${minFileName}`);
    
    // Log size comparison
    const originalSize = fs.statSync(filePath).size;
    const minifiedSize = fs.statSync(outputPath).size;
    const ratio = ((minifiedSize / originalSize) * 100).toFixed(2);
    console.log(`  Original: ${(originalSize / 1024).toFixed(2)}KB, Minified: ${(minifiedSize / 1024).toFixed(2)}KB (${ratio}% of original)`);
    
    // Count lines to verify
    const lineCount = result.code.split('\n').length;
    console.log(`  Line count: ${lineCount}`);
  } catch (error) {
    console.error(`✗ Error minifying ${fileName}:`, error);
  }
}

function minifyCSS(filePath) {
  const outputPath = filePath.replace('.css', '.min.css');
  const fileName = path.basename(filePath);
  const minFileName = path.basename(outputPath);
  
  try {
    const source = fs.readFileSync(filePath);
    const result = transform({
      filename: filePath,
      code: source,
      minify: true,
      sourceMap: false,
    });
    
    fs.writeFileSync(outputPath, result.code);
    console.log(`✓ Minified CSS: ${fileName} → ${minFileName}`);
    
    // Log size comparison
    const originalSize = fs.statSync(filePath).size;
    const minifiedSize = fs.statSync(outputPath).size;
    const ratio = ((minifiedSize / originalSize) * 100).toFixed(2);
    console.log(`  Original: ${(originalSize / 1024).toFixed(2)}KB, Minified: ${(minifiedSize / 1024).toFixed(2)}KB (${ratio}% of original)`);
  } catch (error) {
    console.error(`✗ Error minifying ${fileName}:`, error);
  }
}

async function build() {
  console.log('🚀 Starting build process...');
  
  // Ensure directories exist
  await fs.ensureDir(OUTPUT_DIR);
  
  // Find all JS files
  const jsFiles = glob.sync(`${STATIC_DIR}/**/*.js`, { ignore: [`${STATIC_DIR}/**/*.min.js`] });
  
  // Find all CSS files
  const cssFiles = glob.sync(`${STATIC_DIR}/**/*.css`, { ignore: [`${STATIC_DIR}/**/*.min.css`] });
  
  console.log(`Found ${jsFiles.length} JavaScript files and ${cssFiles.length} CSS files to minify.`);
  
  // Minify all JS files
  const jsPromises = jsFiles.map(minifyJavaScript);
  await Promise.all(jsPromises);
  
  // Minify all CSS files
  cssFiles.forEach(minifyCSS);
  
  console.log('✅ Build completed successfully!');
}

// Execute the build function
build().catch(error => {
  console.error('❌ Build failed:', error);
  process.exit(1);
}); 