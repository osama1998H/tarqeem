/**
 * Test Suite Index for Tarqeem VS Code Extension
 * فهرس مجموعة الاختبارات لإضافة ترقيم
 */

import * as path from 'path';
import Mocha from 'mocha';
import * as fs from 'fs';

export async function run(): Promise<void> {
    // Create the mocha test
    const mocha = new Mocha({
        ui: 'tdd',
        color: true,
        timeout: 10000,
    });

    const testsRoot = path.resolve(__dirname, '..');

    // Find test files manually using fs
    const files = findTestFiles(testsRoot);

    // Add files to the test suite
    files.forEach((f: string) => mocha.addFile(f));

    return new Promise((resolve, reject) => {
        try {
            // Run the mocha test
            mocha.run((failures: number) => {
                if (failures > 0) {
                    reject(new Error(`${failures} tests failed.`));
                } else {
                    resolve();
                }
            });
        } catch (err) {
            console.error(err);
            reject(err);
        }
    });
}

/**
 * Recursively find all test files
 */
function findTestFiles(dir: string): string[] {
    const files: string[] = [];

    const items = fs.readdirSync(dir);
    for (const item of items) {
        const fullPath = path.join(dir, item);
        const stat = fs.statSync(fullPath);

        if (stat.isDirectory()) {
            files.push(...findTestFiles(fullPath));
        } else if (item.endsWith('.test.js')) {
            files.push(fullPath);
        }
    }

    return files;
}
