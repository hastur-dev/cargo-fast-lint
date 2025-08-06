// Rust analyzer bridge to integrate cargo-fl logic
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

class RustAnalyzer {
    constructor() {
        this.cargoFlPath = this.findCargoFlExecutable();
    }

    findCargoFlExecutable() {
        // Try to find the cargo-fl executable in the parent directory
        const projectRoot = path.resolve(__dirname, '..');
        const targetPath = path.join(projectRoot, 'target', 'release', 'cargo-fl.exe');
        const targetDebugPath = path.join(projectRoot, 'target', 'debug', 'cargo-fl.exe');
        const unixTargetPath = path.join(projectRoot, 'target', 'release', 'cargo-fl');
        const unixTargetDebugPath = path.join(projectRoot, 'target', 'debug', 'cargo-fl');
        
        if (fs.existsSync(targetPath)) return targetPath;
        if (fs.existsSync(targetDebugPath)) return targetDebugPath;
        if (fs.existsSync(unixTargetPath)) return unixTargetPath;
        if (fs.existsSync(unixTargetDebugPath)) return unixTargetDebugPath;
        
        // Fallback to system PATH
        return 'cargo-fl';
    }

    async analyzeFile(filePath, content) {
        return new Promise((resolve, reject) => {
            if (!this.cargoFlPath) {
                // Fallback to built-in rules if cargo-fl not found
                resolve(this.fallbackAnalysis(content, filePath));
                return;
            }

            // Write content to temporary file for analysis
            const tempFile = path.join(__dirname, 'temp_analysis.rs');
            fs.writeFileSync(tempFile, content);

            const process = spawn(this.cargoFlPath, ['--json', tempFile], {
                stdio: ['pipe', 'pipe', 'pipe']
            });

            let stdout = '';
            let stderr = '';

            process.stdout.on('data', (data) => {
                stdout += data.toString();
            });

            process.stderr.on('data', (data) => {
                stderr += data.toString();
            });

            process.on('close', (code) => {
                // Clean up temp file
                try {
                    fs.unlinkSync(tempFile);
                } catch (e) {
                    // Ignore cleanup errors
                }

                if (code === 0 || code === 1) {
                    try {
                        const results = this.parseCargoFlOutput(stdout);
                        resolve(results);
                    } catch (error) {
                        // Fallback if parsing fails
                        resolve(this.fallbackAnalysis(content, filePath));
                    }
                } else {
                    // Fallback on error
                    resolve(this.fallbackAnalysis(content, filePath));
                }
            });

            process.on('error', (error) => {
                // Fallback if spawn fails
                resolve(this.fallbackAnalysis(content, filePath));
            });
        });
    }

    parseCargoFlOutput(output) {
        const issues = [];
        const lines = output.trim().split('\n').filter(line => line.trim());

        for (const line of lines) {
            try {
                const result = JSON.parse(line);
                if (result.issues) {
                    issues.push(...result.issues);
                } else if (result.rule && result.message) {
                    // Single issue format
                    issues.push(result);
                }
            } catch (e) {
                // Skip invalid JSON lines
                continue;
            }
        }

        return issues;
    }

    fallbackAnalysis(content, filePath) {
        const issues = [];
        const lines = content.split('\n');

        lines.forEach((line, lineIndex) => {
            const lineNumber = lineIndex + 1;
            const lineStart = content.split('\n').slice(0, lineIndex).join('\n').length + (lineIndex > 0 ? 1 : 0);

            // Rule: Detect unused variables (simple regex-based check)
            const unusedVarMatch = line.match(/let\\s+(_?\\w+)\\s*=/);
            if (unusedVarMatch && !unusedVarMatch[1].startsWith('_')) {
                issues.push({
                    rule: 'unused_variable',
                    severity: 'warning',
                    message: `Variable '${unusedVarMatch[1]}' appears to be unused. Consider prefixing with underscore.`,
                    location: {
                        line: lineNumber,
                        column: line.indexOf(unusedVarMatch[1]) + 1,
                        end_line: lineNumber,
                        end_column: line.indexOf(unusedVarMatch[1]) + unusedVarMatch[1].length + 1
                    }
                });
            }

            // Rule: Detect long lines
            if (line.length > 100) {
                issues.push({
                    rule: 'line_too_long',
                    severity: 'info',
                    message: `Line too long (${line.length} > 100 characters)`,
                    location: {
                        line: lineNumber,
                        column: 101,
                        end_line: lineNumber,
                        end_column: line.length + 1
                    }
                });
            }

            // Rule: Detect TODO comments
            if (line.includes('TODO') || line.includes('FIXME')) {
                const todoIndex = Math.max(line.indexOf('TODO'), line.indexOf('FIXME'));
                issues.push({
                    rule: 'todo_comment',
                    severity: 'info',
                    message: 'TODO comment found',
                    location: {
                        line: lineNumber,
                        column: todoIndex + 1,
                        end_line: lineNumber,
                        end_column: line.length + 1
                    }
                });
            }

            // Rule: Detect unsafe blocks
            if (line.includes('unsafe')) {
                const unsafeIndex = line.indexOf('unsafe');
                issues.push({
                    rule: 'unsafe_code',
                    severity: 'warning',
                    message: 'Unsafe code detected. Ensure safety invariants are maintained.',
                    location: {
                        line: lineNumber,
                        column: unsafeIndex + 1,
                        end_line: lineNumber,
                        end_column: unsafeIndex + 7
                    }
                });
            }

            // Rule: Detect missing documentation
            if (line.match(/^\\s*(pub\\s+)?(fn|struct|enum|impl)\\s+\\w+/) && lineIndex > 0) {
                const prevLine = lines[lineIndex - 1] || '';
                if (!prevLine.trim().startsWith('///') && !prevLine.trim().startsWith('//!')) {
                    issues.push({
                        rule: 'missing_docs',
                        severity: 'info',
                        message: 'Public item should have documentation',
                        location: {
                            line: lineNumber,
                            column: 1,
                            end_line: lineNumber,
                            end_column: line.length + 1
                        }
                    });
                }
            }

            // Rule: Detect naming convention violations
            const structMatch = line.match(/struct\\s+(\\w+)/);
            if (structMatch && !this.isPascalCase(structMatch[1])) {
                issues.push({
                    rule: 'naming_convention',
                    severity: 'warning',
                    message: `Struct name '${structMatch[1]}' should be PascalCase`,
                    location: {
                        line: lineNumber,
                        column: line.indexOf(structMatch[1]) + 1,
                        end_line: lineNumber,
                        end_column: line.indexOf(structMatch[1]) + structMatch[1].length + 1
                    }
                });
            }

            const fnMatch = line.match(/fn\\s+(\\w+)/);
            if (fnMatch && !this.isSnakeCase(fnMatch[1])) {
                issues.push({
                    rule: 'naming_convention',
                    severity: 'warning',
                    message: `Function name '${fnMatch[1]}' should be snake_case`,
                    location: {
                        line: lineNumber,
                        column: line.indexOf(fnMatch[1]) + 1,
                        end_line: lineNumber,
                        end_column: line.indexOf(fnMatch[1]) + fnMatch[1].length + 1
                    }
                });
            }
        });

        return issues;
    }

    isPascalCase(str) {
        return /^[A-Z][a-zA-Z0-9]*$/.test(str);
    }

    isSnakeCase(str) {
        return /^[a-z][a-z0-9_]*$/.test(str);
    }
}

module.exports = RustAnalyzer;