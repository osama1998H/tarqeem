/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Copyright (c) Sahari Contributors. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { RunOnceScheduler } from '../../../../base/common/async.js';
import { Disposable } from '../../../../base/common/lifecycle.js';
import { ICodeEditor } from '../../../browser/editorBrowser.js';
import { EditorContributionInstantiation, registerEditorContribution } from '../../../browser/editorExtensions.js';
import { IEditorContribution, IEditorDecorationsCollection } from '../../../common/editorCommon.js';
import { IModelDeltaDecoration, TextDirection, TrackedRangeStickiness } from '../../../common/model.js';
import { Range } from '../../../common/core/range.js';
import { IConfigurationService } from '../../../../platform/configuration/common/configuration.js';

/**
 * Unicode ranges for Arabic script characters
 * صحاري - Sahari: Auto-RTL Detection for Arabic Content
 */
const ARABIC_UNICODE_RANGES = [
	[0x0600, 0x06FF],  // Arabic
	[0x0750, 0x077F],  // Arabic Supplement
	[0x08A0, 0x08FF],  // Arabic Extended-A
	[0xFB50, 0xFDFF],  // Arabic Presentation Forms-A
	[0xFE70, 0xFEFF],  // Arabic Presentation Forms-B
];

/**
 * Unicode ranges for Hebrew script characters
 */
const HEBREW_UNICODE_RANGES = [
	[0x0590, 0x05FF],  // Hebrew
	[0xFB00, 0xFB4F],  // Hebrew Presentation Forms
];

/**
 * Check if a character is an RTL character (Arabic or Hebrew)
 */
function isRTLChar(charCode: number): boolean {
	for (const [start, end] of ARABIC_UNICODE_RANGES) {
		if (charCode >= start && charCode <= end) {
			return true;
		}
	}
	for (const [start, end] of HEBREW_UNICODE_RANGES) {
		if (charCode >= start && charCode <= end) {
			return true;
		}
	}
	return false;
}

/**
 * Check if a line should be RTL based on its content
 * Uses first strong character algorithm (simplified UAX#9)
 */
function shouldLineBeRTL(lineContent: string): boolean {
	for (let i = 0; i < lineContent.length; i++) {
		const charCode = lineContent.charCodeAt(i);

		// Skip whitespace and neutral characters
		if (charCode <= 0x20 || (charCode >= 0x21 && charCode <= 0x2F) ||
			(charCode >= 0x3A && charCode <= 0x40) || (charCode >= 0x5B && charCode <= 0x60) ||
			(charCode >= 0x7B && charCode <= 0x7E)) {
			continue;
		}

		// Check for RTL characters
		if (isRTLChar(charCode)) {
			return true;
		}

		// Check for LTR Latin characters (a-z, A-Z)
		if ((charCode >= 0x41 && charCode <= 0x5A) || (charCode >= 0x61 && charCode <= 0x7A)) {
			return false;
		}

		// Check for digits - neutral, continue
		if (charCode >= 0x30 && charCode <= 0x39) {
			continue;
		}
	}

	return false;
}

/**
 * RTL Auto-Detection Controller
 * Automatically applies RTL text direction to lines containing Arabic/Hebrew content
 */
export class RTLDetectionController extends Disposable implements IEditorContribution {
	public static readonly ID = 'editor.contrib.rtlDetectionController';

	public static get(editor: ICodeEditor): RTLDetectionController | null {
		return editor.getContribution<RTLDetectionController>(RTLDetectionController.ID);
	}

	private readonly _editor: ICodeEditor;
	private readonly _decorations: IEditorDecorationsCollection;
	private readonly _updateRTLSoon: RunOnceScheduler;
	private _enabled: boolean = true;
	private _lastVersionId: number = 0;

	constructor(
		editor: ICodeEditor,
		@IConfigurationService private readonly _configurationService: IConfigurationService
	) {
		super();
		this._editor = editor;
		this._decorations = this._editor.createDecorationsCollection();
		this._updateRTLSoon = this._register(new RunOnceScheduler(() => this._updateRTLDecorations(), 100));

		// Read initial configuration
		this._readConfiguration();

		// Schedule initial update
		this._updateRTLSoon.schedule();

		// Listen to model changes
		this._register(editor.onDidChangeModelContent(() => {
			if (this._enabled) {
				this._updateRTLSoon.schedule();
			}
		}));

		// Listen to model switch
		this._register(editor.onDidChangeModel(() => {
			this._lastVersionId = 0;
			if (this._enabled) {
				this._updateRTLSoon.schedule();
			}
		}));

		// Listen to configuration changes
		this._register(this._configurationService.onDidChangeConfiguration(e => {
			if (e.affectsConfiguration('sahari.editor.autoRTL') ||
				e.affectsConfiguration('editor.autoRTL')) {
				this._readConfiguration();
			}
		}));
	}

	private _readConfiguration(): void {
		// Check for sahari-specific setting first, then fallback to editor setting
		const sahariConfig = this._configurationService.getValue<boolean>('sahari.editor.autoRTL');
		const editorConfig = this._configurationService.getValue<boolean>('editor.autoRTL');

		this._enabled = sahariConfig ?? editorConfig ?? true;

		if (this._enabled) {
			this._updateRTLSoon.schedule();
		} else {
			this._decorations.clear();
		}
	}

	private _updateRTLDecorations(): void {
		if (!this._enabled) {
			return;
		}

		const model = this._editor.getModel();
		if (!model) {
			this._decorations.clear();
			return;
		}

		const versionId = model.getVersionId();
		if (versionId === this._lastVersionId) {
			return;
		}
		this._lastVersionId = versionId;

		const decorations: IModelDeltaDecoration[] = [];
		const lineCount = model.getLineCount();

		for (let lineNumber = 1; lineNumber <= lineCount; lineNumber++) {
			const lineContent = model.getLineContent(lineNumber);

			if (shouldLineBeRTL(lineContent)) {
				decorations.push({
					range: new Range(lineNumber, 1, lineNumber, 1),
					options: {
						description: 'rtl-auto-detection',
						stickiness: TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
						textDirection: TextDirection.RTL
					}
				});
			}
		}

		this._decorations.set(decorations);
	}

	/**
	 * Manually trigger RTL detection update
	 */
	public updateRTL(): void {
		this._lastVersionId = 0;
		this._updateRTLSoon.schedule();
	}

	/**
	 * Enable or disable auto-RTL detection
	 */
	public setEnabled(enabled: boolean): void {
		this._enabled = enabled;
		if (enabled) {
			this._updateRTLSoon.schedule();
		} else {
			this._decorations.clear();
		}
	}

	/**
	 * Check if auto-RTL detection is enabled
	 */
	public isEnabled(): boolean {
		return this._enabled;
	}
}

registerEditorContribution(RTLDetectionController.ID, RTLDetectionController, EditorContributionInstantiation.AfterFirstRender);
