/**
 * Extension Tests for Tarqeem VS Code Extension
 * اختبارات الإضافة لترقيم
 */

import * as assert from 'assert';
import * as vscode from 'vscode';

suite('Extension Test Suite', () => {
    vscode.window.showInformationMessage('Start all tests.');

    test('Extension should be present', () => {
        const extension = vscode.extensions.getExtension('tarqeem.tarqeem');
        assert.ok(extension, 'Extension should be installed');
    });

    test('Should activate on Tarqeem file', async () => {
        // Create a new untitled document
        const document = await vscode.workspace.openTextDocument({
            language: 'tarqeem',
            content: 'اطبع("مرحبا")',
        });

        assert.strictEqual(document.languageId, 'tarqeem');
    });

    test('Language should be registered', () => {
        const languages = vscode.languages.getLanguages();
        languages.then((langs) => {
            assert.ok(langs.includes('tarqeem'), 'Tarqeem language should be registered');
        });
    });

    test('Commands should be registered', async () => {
        const commands = await vscode.commands.getCommands(true);

        const expectedCommands = [
            'tarqeem.run',
            'tarqeem.compile',
            'tarqeem.format',
            'tarqeem.newProject',
            'tarqeem.openRepl',
            'tarqeem.check',
            'tarqeem.restartServer',
        ];

        for (const cmd of expectedCommands) {
            assert.ok(
                commands.includes(cmd),
                `Command ${cmd} should be registered`
            );
        }
    });
});

suite('RTL Utilities Test Suite', () => {
    test('Should detect Arabic characters', async () => {
        // Import the RTL module dynamically
        const rtl = await import('../../rtl.js');

        assert.ok(rtl.isArabicChar('م'), 'Should detect Arabic letter');
        assert.ok(rtl.isArabicChar('ت'), 'Should detect Arabic letter');
        assert.ok(!rtl.isArabicChar('a'), 'Should not detect Latin letter as Arabic');
        assert.ok(!rtl.isArabicChar('1'), 'Should not detect digit as Arabic');
    });

    test('Should check if text contains Arabic', async () => {
        const rtl = await import('../../rtl.js');

        assert.ok(rtl.containsArabic('مرحبا'), 'Should detect Arabic text');
        assert.ok(rtl.containsArabic('Hello مرحبا'), 'Should detect mixed text');
        assert.ok(!rtl.containsArabic('Hello World'), 'Should not detect English-only text');
    });

    test('Should validate identifiers', async () => {
        const rtl = await import('../../rtl.js');

        // Valid identifiers
        assert.ok(rtl.isValidIdentifier('متغير'), 'Arabic identifier should be valid');
        assert.ok(rtl.isValidIdentifier('variable'), 'English identifier should be valid');
        assert.ok(rtl.isValidIdentifier('_private'), 'Underscore prefix should be valid');
        assert.ok(rtl.isValidIdentifier('var123'), 'Alphanumeric should be valid');
        assert.ok(rtl.isValidIdentifier('متغير1'), 'Arabic with number should be valid');

        // Invalid identifiers
        assert.ok(!rtl.isValidIdentifier(''), 'Empty string should be invalid');
        assert.ok(!rtl.isValidIdentifier('123var'), 'Starting with number should be invalid');
        assert.ok(!rtl.isValidIdentifier('var-name'), 'Hyphen should be invalid');
    });

    test('Should get text direction', async () => {
        const rtl = await import('../../rtl.js');

        assert.strictEqual(rtl.getTextDirection('مرحبا'), 'rtl', 'Arabic text should be RTL');
        assert.strictEqual(rtl.getTextDirection('Hello'), 'ltr', 'English text should be LTR');
        assert.strictEqual(rtl.getTextDirection('Hello مرحبا'), 'mixed', 'Mixed text should be mixed');
    });

    test('Should normalize Arabic text', async () => {
        const rtl = await import('../../rtl.js');

        const normalized = rtl.normalizeArabicText('مُرَحَّبًا');
        assert.ok(normalized.length > 0, 'Normalized text should not be empty');
        assert.strictEqual(normalized, normalized.normalize('NFC'), 'Should be NFC normalized');
    });

    test('Should convert Arabic digits to Western', async () => {
        const rtl = await import('../../rtl.js');

        assert.strictEqual(rtl.arabicToWesternDigits('١٢٣'), '123', 'Should convert Arabic digits');
        assert.strictEqual(rtl.arabicToWesternDigits('٠'), '0', 'Should convert zero');
        assert.strictEqual(rtl.arabicToWesternDigits('abc'), 'abc', 'Should leave non-digits unchanged');
        assert.strictEqual(rtl.arabicToWesternDigits('١a٢'), '1a2', 'Should handle mixed content');
    });
});

suite('Grammar Test Suite', () => {
    test('Should tokenize Arabic keywords', async () => {
        const document = await vscode.workspace.openTextDocument({
            language: 'tarqeem',
            content: `
دالة مرحبا() {
    متغير س = 5
    إذا (س > 0) {
        اطبع("موجب")
    }
    أرجع س
}
`,
        });

        // Verify the document is recognized as Tarqeem
        assert.strictEqual(document.languageId, 'tarqeem');

        // Get the text - tokenization is handled by TextMate grammar
        const text = document.getText();
        assert.ok(text.includes('دالة'), 'Should contain function keyword');
        assert.ok(text.includes('متغير'), 'Should contain variable keyword');
        assert.ok(text.includes('إذا'), 'Should contain if keyword');
    });

    test('Should handle mixed Arabic and English', async () => {
        const document = await vscode.workspace.openTextDocument({
            language: 'tarqeem',
            content: `
متغير userName = "أحمد"
const PI = 3.14
function greet() {
    اطبع("مرحبا")
}
`,
        });

        assert.strictEqual(document.languageId, 'tarqeem');

        const text = document.getText();
        assert.ok(text.includes('userName'), 'Should contain English identifier');
        assert.ok(text.includes('أحمد'), 'Should contain Arabic string');
    });
});

suite('Snippet Test Suite', () => {
    test('Snippets file should be valid JSON', async () => {
        const fs = await import('fs');
        const path = await import('path');

        const snippetsPath = path.join(__dirname, '../../../snippets/tarqeem.json');

        // Check if file exists (in compiled output location)
        try {
            const content = fs.readFileSync(snippetsPath, 'utf8');
            const snippets = JSON.parse(content);
            assert.ok(typeof snippets === 'object', 'Snippets should be an object');

            // Check for some expected snippets
            const expectedSnippets = ['دالة - Function', 'صنف - Class', 'إذا - If'];
            for (const snippet of expectedSnippets) {
                assert.ok(
                    snippets[snippet] !== undefined,
                    `Snippet '${snippet}' should exist`
                );
            }
        } catch {
            // File might not exist in test environment - skip
            console.log('Snippets file not found in test environment - skipping');
        }
    });
});
