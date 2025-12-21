<div dir="rtl" align="right">

# صحاري - Sahari Editor
## خطة تطوير محرر الأكواد العربي

</div>

# Sahari (صحاري) - Arabic-First Code Editor

**Project Vision**: Fork VS Code to create an Arabic-first code editor with native RTL support, optimized for the Tarqeem programming language.

---

## Executive Summary

### Good News: RTL Support Already Merged!

As of **July 14, 2025**, Microsoft merged [PR #255455](https://github.com/microsoft/vscode/pull/255455) which adds RTL support to Monaco Editor. This dramatically reduces our development effort.

### What We Need to Do

1. Fork VS Code (post-July 2025 version with RTL support)
2. Enable RTL by default for Arabic content
3. Rebrand to "Sahari" (صحاري)
4. Bundle Tarqeem extension and Arabic fonts
5. Optimize UX for Arabic developers

---

## Phase 1: Foundation (Week 1-2)

### 1.1 Repository Setup

```bash
# Clone VS Code source
git clone https://github.com/microsoft/vscode.git sahari
cd sahari

# Ensure we have the RTL PR (merged July 14, 2025)
git log --oneline --grep="RTL"

# Create Sahari branch
git checkout -b sahari/main
```

### 1.2 Build Environment

**Prerequisites:**
- Node.js 18.x or 20.x
- Python 3.x
- Git
- yarn (VS Code uses yarn)
- C++ Build Tools (for native modules)

**Build Steps:**
```bash
# Install dependencies
yarn

# Build
yarn compile

# Run development version
./scripts/code.sh  # Linux/macOS
```

### 1.3 Verify RTL Support

Test that the merged RTL functionality works:
- Create Arabic text file
- Verify cursor moves correctly (right-to-left)
- Verify Home/End keys work correctly
- Test mixed Arabic/English text

---

## Phase 2: RTL Enhancement (Week 2-3)

### 2.1 Default RTL for Arabic Files

The merged PR uses decorations for RTL. We need to:

**Auto-detect Arabic content and apply RTL:**

```typescript
// src/vs/editor/contrib/rtl/rtlDetector.ts (new file)
export function detectTextDirection(text: string): 'ltr' | 'rtl' {
    const arabicRegex = /[\u0600-\u06FF\u0750-\u077F\u08A0-\u08FF]/;
    const firstSignificantChar = text.match(/\S/);
    if (firstSignificantChar && arabicRegex.test(firstSignificantChar[0])) {
        return 'rtl';
    }
    return 'ltr';
}
```

**Key files to modify:**
- `src/vs/editor/common/model/textModel.ts` - Add RTL detection on content change
- `src/vs/editor/browser/view/viewImpl.ts` - Apply RTL decorations automatically
- `src/vs/editor/common/config/editorOptions.ts` - Add `defaultTextDirection` option

### 2.2 Editor Configuration

Add new settings for Arabic support:

```json
{
    "sahari.editor.defaultDirection": "auto",  // auto | rtl | ltr
    "sahari.editor.arabicFontFamily": "Amiri, 'Noto Sans Arabic', monospace",
    "sahari.editor.numbersStyle": "arabic",    // arabic (٠١٢) | western (012)
    "sahari.ui.direction": "rtl"               // RTL UI chrome
}
```

### 2.3 UI Direction (Optional)

Make the entire UI RTL:
- Sidebar on the right
- Menus right-aligned
- Activity bar on the right

**Files:**
- `src/vs/workbench/browser/layout.ts`
- `src/vs/base/browser/ui/` (various UI components)

---

## Phase 3: Branding (Week 3-4)

### 3.1 Name and Identity

| Item | Original | Sahari |
|------|----------|--------|
| Name | Visual Studio Code | صحاري - Sahari |
| Tagline | Code editing. Redefined. | برمجة بلغتك - Code in Your Language |
| App ID | `code` | `sahari` |
| File Association | `.code-workspace` | `.sahari-workspace` |

### 3.2 Files to Modify for Branding

```
product.json                    # Main product configuration
resources/linux/code.desktop    # Linux desktop entry
resources/win32/code.ico        # Windows icon
resources/darwin/code.icns      # macOS icon
src/vs/workbench/browser/parts/titlebar/
src/vs/code/electron-main/app.ts
```

### 3.3 Logo and Icons

Create Arabic-themed branding:
- App icon: Desert/oasis theme with Arabic calligraphy
- Splash screen: صحاري logo
- About dialog: Arabic credits

### 3.4 Localization

Prioritize Arabic localization:
- `i18n/` folder contains translations
- Ensure all UI strings have Arabic translations
- RTL-aware layout for dialogs and panels

---

## Phase 4: Tarqeem Integration (Week 4-5)

### 4.1 Bundle Tarqeem Extension

Pre-install the Tarqeem extension:

```json
// product.json
{
    "builtInExtensions": [
        {
            "name": "tarqeem.tarqeem",
            "version": "0.1.0",
            "repo": "https://github.com/osama1998H/tarqeem",
            "path": "vscode-tarqeem"
        }
    ]
}
```

### 4.2 Bundle Arabic Fonts

Include high-quality Arabic programming fonts:

1. **Amiri** - Beautiful Arabic typeface
2. **Noto Sans Arabic** - Google's comprehensive Arabic font
3. **Cairo** - Modern Arabic font
4. **IBM Plex Arabic** - Monospace-friendly

**Installation:**
```
resources/fonts/
├── Amiri-Regular.ttf
├── NotoSansArabic-Regular.ttf
├── Cairo-Regular.ttf
└── IBMPlexArabic-Regular.ttf
```

**CSS Integration:**
```css
/* src/vs/workbench/browser/media/style.css */
@font-face {
    font-family: 'Sahari Arabic';
    src: url('./fonts/Amiri-Regular.ttf') format('truetype');
}
```

### 4.3 Default Settings for Arabic Development

```json
{
    "editor.fontFamily": "'Amiri', 'Noto Sans Arabic', 'Fira Code', monospace",
    "editor.fontSize": 16,
    "editor.lineHeight": 1.8,
    "editor.unicodeHighlight.ambiguousCharacters": false,
    "editor.unicodeHighlight.nonBasicASCII": false,
    "[tarqeem]": {
        "editor.defaultTextDirection": "auto"
    }
}
```

---

## Phase 5: Build & Distribution (Week 5-6)

### 5.1 Build Scripts

```bash
# Build for all platforms
yarn gulp vscode-linux-x64
yarn gulp vscode-darwin-x64
yarn gulp vscode-darwin-arm64
yarn gulp vscode-win32-x64

# Create installers
yarn gulp vscode-linux-x64-build-deb
yarn gulp vscode-linux-x64-build-rpm
yarn gulp vscode-darwin-x64-dmg
yarn gulp vscode-win32-x64-setup
```

### 5.2 Distribution Channels

1. **GitHub Releases** - Primary distribution
2. **Website** - sahari.dev (future)
3. **Package Managers**:
   - Snap Store (Linux)
   - Homebrew (macOS)
   - Chocolatey (Windows)

### 5.3 Auto-Update

Configure update server:
```json
// product.json
{
    "updateUrl": "https://update.sahari.dev",
    "quality": "stable"
}
```

---

## Architecture Overview

```
sahari/
├── src/
│   └── vs/
│       ├── base/              # Utilities (minimal changes)
│       ├── platform/          # Services (RTL service)
│       ├── editor/            # Monaco (RTL already merged!)
│       │   └── contrib/
│       │       └── rtl/       # Our RTL enhancements
│       ├── workbench/         # UI (RTL layout changes)
│       └── code/              # Electron app (branding)
├── extensions/
│   └── tarqeem/               # Built-in Tarqeem extension
├── resources/
│   ├── fonts/                 # Arabic fonts
│   └── branding/              # Sahari logos/icons
└── product.json               # Product configuration
```

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| VS Code updates break our changes | High | High | Maintain minimal diff; rebase regularly |
| RTL bugs in merged PR | Medium | Medium | Test thoroughly; contribute fixes upstream |
| Build complexity | Medium | Low | Document build process; use CI/CD |
| Licensing issues | Low | High | VS Code is MIT; ensure compliance |
| Extension compatibility | Medium | Medium | Test popular extensions |

---

## Success Metrics

1. **RTL Writing**: Cursor moves correctly for Arabic text
2. **Font Rendering**: Arabic text displays beautifully
3. **Performance**: No degradation vs stock VS Code
4. **Compatibility**: Tarqeem extension works fully
5. **User Experience**: Arabic developers find it intuitive

---

## Timeline Summary

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| 1. Foundation | 2 weeks | Building VS Code, RTL verified |
| 2. RTL Enhancement | 1 week | Auto-RTL detection, settings |
| 3. Branding | 1 week | Sahari identity complete |
| 4. Tarqeem Integration | 1 week | Extension + fonts bundled |
| 5. Distribution | 1 week | Installers for all platforms |
| **Total** | **6 weeks** | **Sahari v0.1.0 Release** |

---

## Progress Tracker

### Phase 1: Foundation ✅ COMPLETE (December 21, 2025)

- [x] Clone VS Code repository (v1.108.0)
- [x] Set up build environment (Node.js 22.21.1, npm)
- [x] Verify RTL PR #255455 is included
- [x] Create `sahari/main` branch
- [x] Install dependencies (npm install)
- [x] Compile VS Code source (0 errors after fixes)
- [x] Fix TypeScript compilation issues (terminalProcess.ts)

**Key Findings:**
- RTL support is fully integrated via `TextDirection` enum
- Cursor movement, mouse handling, and line rendering all support RTL
- VS Code now uses npm (not yarn) for package management
- Required system packages: `libxkbfile-dev`, `libx11-dev`, `libsecret-1-dev`, `libkrb5-dev`

### Next Steps (Phase 2)

1. [ ] Implement auto-RTL detection for Arabic content
2. [ ] Add Sahari-specific editor settings
3. [ ] Begin branding changes (product.json)

---

## Resources

- [VS Code Source Code](https://github.com/microsoft/vscode)
- [VS Code Source Organization](https://github.com/microsoft/vscode/wiki/source-code-organization)
- [RTL Support PR #255455](https://github.com/microsoft/vscode/pull/255455)
- [Monaco Editor](https://github.com/microsoft/monaco-editor)
- [VS Code Build Instructions](https://github.com/microsoft/vscode/wiki/How-to-Contribute)

---

<div dir="rtl" align="right">

## عن الاسم

**صحاري** (Sahari) - جمع صحراء، تمثل الفضاء الواسع للإبداع والبرمجة. كما أن الصحراء العربية كانت مهد الحضارة، نريد لصحاري أن تكون مهد البرمجة العربية.

</div>
