<div dir="rtl" align="right">

# دليل تثبيت ترقيم

دليل شامل لتثبيت مترجم ترقيم على أنظمة التشغيل المختلفة.

---

## فهرس المحتويات

1. [المتطلبات](#المتطلبات)
2. [التثبيت على لينكس](#التثبيت-على-لينكس)
3. [التثبيت على macOS](#التثبيت-على-macos)
4. [التثبيت على ويندوز](#التثبيت-على-ويندوز)
5. [التحقق من التثبيت](#التحقق-من-التثبيت)
6. [إلغاء التثبيت](#إلغاء-التثبيت)
7. [استكشاف الأخطاء](#استكشاف-الأخطاء)

---

## المتطلبات

### المتطلبات الأساسية

| المتطلب | الإصدار الأدنى | الوصف |
|---------|---------------|-------|
| **رست (Rust)** | 1.70+ | لغة البرمجة المستخدمة لبناء ترقيم |
| **LLVM** | 14+ | واجهة توليد الكود الأصلي |
| **Clang** | 14+ | مُترجم C للربط |
| **جِت (Git)** | 2.0+ | لاستنساخ المستودع |

### تثبيت المتطلبات

#### لينكس (Ubuntu/Debian)

```bash
# تثبيت أدوات البناء الأساسية
sudo apt update
sudo apt install build-essential git curl

# تثبيت LLVM و Clang
sudo apt install llvm-14 clang-14

# تثبيت Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### لينكس (Fedora/RHEL)

```bash
# تثبيت أدوات البناء
sudo dnf groupinstall "Development Tools"
sudo dnf install git curl

# تثبيت LLVM و Clang
sudo dnf install llvm clang

# تثبيت Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### لينكس (Arch)

```bash
# تثبيت أدوات البناء و LLVM
sudo pacman -S base-devel git llvm clang

# تثبيت Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### macOS

```bash
# تثبيت Homebrew (إن لم يكن مثبتاً)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# تثبيت LLVM
brew install llvm

# تثبيت Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### ويندوز

1. **تثبيت Visual Studio Build Tools**:
   - حمّل [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)
   - اختر "Desktop development with C++"

2. **تثبيت LLVM**:
   - حمّل من [LLVM Releases](https://releases.llvm.org/)
   - أو استخدم `winget install LLVM.LLVM`

3. **تثبيت Rust**:
   - حمّل وشغّل [rustup-init.exe](https://rustup.rs/)

4. **تثبيت Git**:
   - حمّل من [git-scm.com](https://git-scm.com/download/win)

---

## التثبيت على لينكس

### الطريقة ١: التثبيت السريع (موصى بها)

```bash
# ١. استنساخ المستودع
git clone https://github.com/osama1998H/tarqeem.git
cd tarqeem

# ٢. تشغيل سكربت التثبيت
chmod +x install.sh
./install.sh

# ٣. إضافة المتغيرات للـ shell
# أضف الأسطر التالية لملف ~/.bashrc أو ~/.zshrc:

export TARQEEM_HOME="$HOME/.tarqeem"
export PATH="$TARQEEM_HOME/bin:$PATH"

# ٤. تحديث الجلسة الحالية
source ~/.bashrc  # أو source ~/.zshrc
```

### الطريقة ٢: التثبيت باستخدام Makefile

```bash
# استنساخ المستودع
git clone https://github.com/osama1998H/tarqeem.git
cd tarqeem

# البناء والتثبيت
make build
make install

# إضافة المتغيرات (كما في الطريقة ١)
echo 'export TARQEEM_HOME="$HOME/.tarqeem"' >> ~/.bashrc
echo 'export PATH="$TARQEEM_HOME/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### الطريقة ٣: التثبيت في مسار مخصص

```bash
# تحديد مسار التثبيت المخصص
export TARQEEM_HOME="/opt/tarqeem"

# تشغيل سكربت التثبيت
./install.sh

# أو باستخدام Makefile
make install PREFIX=/opt/tarqeem

# تحديث PATH
echo 'export TARQEEM_HOME="/opt/tarqeem"' >> ~/.bashrc
echo 'export PATH="$TARQEEM_HOME/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### إضافة للـ Shell بشكل دائم

#### Bash (~/.bashrc)

```bash
# إضافة ترقيم للـ PATH
export TARQEEM_HOME="$HOME/.tarqeem"
export PATH="$TARQEEM_HOME/bin:$PATH"
```

#### Zsh (~/.zshrc)

```bash
# إضافة ترقيم للـ PATH
export TARQEEM_HOME="$HOME/.tarqeem"
export PATH="$TARQEEM_HOME/bin:$PATH"
```

#### Fish (~/.config/fish/config.fish)

```fish
# إضافة ترقيم للـ PATH
set -gx TARQEEM_HOME $HOME/.tarqeem
set -gx PATH $TARQEEM_HOME/bin $PATH
```

---

## التثبيت على macOS

### الطريقة ١: التثبيت السريع

```bash
# ١. استنساخ المستودع
git clone https://github.com/osama1998H/tarqeem.git
cd tarqeem

# ٢. تشغيل سكربت التثبيت
chmod +x install.sh
./install.sh

# ٣. إضافة المتغيرات للـ shell
# لـ Zsh (الافتراضي في macOS):
echo 'export TARQEEM_HOME="$HOME/.tarqeem"' >> ~/.zshrc
echo 'export PATH="$TARQEEM_HOME/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

# أو لـ Bash:
echo 'export TARQEEM_HOME="$HOME/.tarqeem"' >> ~/.bash_profile
echo 'export PATH="$TARQEEM_HOME/bin:$PATH"' >> ~/.bash_profile
source ~/.bash_profile
```

### الطريقة ٢: استخدام Makefile

```bash
git clone https://github.com/osama1998H/tarqeem.git
cd tarqeem

make build
make install

# إضافة للـ PATH
echo 'export TARQEEM_HOME="$HOME/.tarqeem"' >> ~/.zshrc
echo 'export PATH="$TARQEEM_HOME/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### ملاحظات خاصة بـ macOS

#### Apple Silicon (M1/M2/M3)

إذا كنت تستخدم Mac بمعالج Apple Silicon، تأكد من:

```bash
# تحقق من أن LLVM مثبت لـ ARM
brew info llvm

# إذا لزم الأمر، أضف LLVM للـ PATH
export PATH="/opt/homebrew/opt/llvm/bin:$PATH"
```

#### Gatekeeper

إذا ظهرت رسالة "لا يمكن التحقق من المطور":

```bash
# السماح بتشغيل tarqeem
xattr -d com.apple.quarantine ~/.tarqeem/bin/tarqeem
```

---

## التثبيت على ويندوز

### الطريقة ١: PowerShell (موصى بها)

```powershell
# افتح PowerShell كمسؤول

# ١. استنساخ المستودع
git clone https://github.com/osama1998H/tarqeem.git
cd tarqeem

# ٢. تشغيل سكربت التثبيت
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
.\install.ps1

# سيقوم السكربت تلقائياً بـ:
# - تثبيت ترقيم في %LocalAppData%\Tarqeem
# - تعيين متغير البيئة TARQEEM_HOME
# - إضافة المسار للـ PATH

# ٣. أعد تشغيل PowerShell أو Terminal لتطبيق التغييرات
```

### الطريقة ٢: التثبيت اليدوي

```powershell
# ١. استنساخ وبناء
git clone https://github.com/osama1998H/tarqeem.git
cd tarqeem
cargo build --release

# ٢. إنشاء مجلد التثبيت
$InstallDir = "$env:LOCALAPPDATA\Tarqeem"
New-Item -ItemType Directory -Force -Path "$InstallDir\bin"
New-Item -ItemType Directory -Force -Path "$InstallDir\lib"
New-Item -ItemType Directory -Force -Path "$InstallDir\stdlib_trq"

# ٣. نسخ الملفات
Copy-Item "target\release\tarqeem.exe" "$InstallDir\bin\"
Copy-Item "target\release\libtrq.a" "$InstallDir\lib\"
Copy-Item -Recurse "stdlib_trq\*" "$InstallDir\stdlib_trq\"

# ٤. تعيين متغيرات البيئة
[Environment]::SetEnvironmentVariable("TARQEEM_HOME", $InstallDir, "User")
$Path = [Environment]::GetEnvironmentVariable("Path", "User")
if ($Path -notlike "*$InstallDir\bin*") {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir\bin;$Path", "User")
}

# ٥. أعد تشغيل Terminal
```

### تثبيت بيئة التطوير على ويندوز

#### تثبيت LLVM عبر Chocolatey

```powershell
# تثبيت Chocolatey (إن لم يكن مثبتاً)
Set-ExecutionPolicy Bypass -Scope Process -Force
[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))

# تثبيت LLVM
choco install llvm
```

#### تثبيت LLVM عبر winget

```powershell
winget install LLVM.LLVM
```

### ملاحظات خاصة بويندوز

1. **Visual Studio Build Tools**: مطلوب لترجمة مكتبة وقت التشغيل
2. **إعادة تشغيل Terminal**: يجب إعادة تشغيل PowerShell أو CMD بعد التثبيت
3. **Windows Defender**: قد يحتاج السماح بتشغيل tarqeem.exe

---

## التحقق من التثبيت

### التحقق الأساسي

```bash
# تحقق من الإصدار
tarqeem --version

# يجب أن يظهر:
# tarqeem 0.1.0 (أو الإصدار الحالي)
```

### اختبار الترجمة

```bash
# إنشاء ملف اختبار
cat > مرحبا.ترقيم << 'EOF'
بسم_الله

اطبع("مرحباً بالعالم!")

الحمد_لله
EOF

# تشغيل البرنامج
tarqeem run مرحبا.ترقيم

# يجب أن يظهر:
# مرحباً بالعالم!
```

### اختبار الترجمة إلى ملف تنفيذي

```bash
# الترجمة
tarqeem compile مرحبا.ترقيم -o مرحبا

# التشغيل
./مرحبا  # أو .\مرحبا.exe على ويندوز

# يجب أن يظهر:
# مرحباً بالعالم!
```

### التحقق من المتغيرات البيئية

```bash
# لينكس/macOS
echo $TARQEEM_HOME
echo $PATH | grep tarqeem

# ويندوز (PowerShell)
echo $env:TARQEEM_HOME
echo $env:Path | Select-String tarqeem
```

---

## إلغاء التثبيت

### لينكس/macOS

```bash
# إزالة مجلد التثبيت
rm -rf ~/.tarqeem

# أو باستخدام Makefile
cd tarqeem
make uninstall

# إزالة الأسطر من ~/.bashrc أو ~/.zshrc:
# export TARQEEM_HOME="$HOME/.tarqeem"
# export PATH="$TARQEEM_HOME/bin:$PATH"
```

### ويندوز

```powershell
# إزالة مجلد التثبيت
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\Tarqeem"

# إزالة متغيرات البيئة
[Environment]::SetEnvironmentVariable("TARQEEM_HOME", $null, "User")

# إزالة من PATH (يدوياً من إعدادات النظام)
```

---

## استكشاف الأخطاء

### الخطأ: "tarqeem: command not found"

**السبب**: المسار غير مُضاف للـ PATH

**الحل**:
```bash
# تحقق من وجود الملف التنفيذي
ls ~/.tarqeem/bin/tarqeem

# أضف للـ PATH
export PATH="$HOME/.tarqeem/bin:$PATH"

# أو أضف للـ shell بشكل دائم
echo 'export PATH="$HOME/.tarqeem/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### الخطأ: "LLVM not found"

**الحل**:
```bash
# لينكس
sudo apt install llvm-14

# macOS
brew install llvm
export PATH="/opt/homebrew/opt/llvm/bin:$PATH"

# ويندوز
winget install LLVM.LLVM
```

### الخطأ: "libtrq.a not found"

**السبب**: مكتبة وقت التشغيل غير موجودة

**الحل**:
```bash
# إعادة بناء المترجم ومكتبة وقت التشغيل
cd tarqeem
cargo build --release

# تحقق من وجود المكتبة
ls target/release/libtrq.a

# إعادة التثبيت
./install.sh
```

### الخطأ: "clang: error: linker command failed"

**السبب**: Clang غير مثبت أو غير متاح

**الحل**:
```bash
# لينكس
sudo apt install clang

# macOS
xcode-select --install

# ويندوز
# تأكد من تثبيت Visual Studio Build Tools
```

### الخطأ على ويندوز: "execution policy"

**الحل**:
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

---

## هيكل التثبيت

بعد التثبيت، سيكون الهيكل كالتالي:

```
~/.tarqeem/                    # أو %LocalAppData%\Tarqeem على ويندوز
├── bin/
│   └── tarqeem               # الملف التنفيذي
├── lib/
│   └── libtrq.a              # مكتبة وقت التشغيل
├── stdlib_trq/               # المكتبة القياسية
│   ├── مجموعات/
│   ├── رياضيات/
│   ├── نص/
│   ├── ملفات/
│   ├── طرفية/
│   ├── وقت/
│   ├── شبكة/
│   └── أخطاء/
└── VERSION                   # ملف الإصدار
```

---

## المساعدة

للحصول على مساعدة إضافية:

```bash
# عرض المساعدة
tarqeem --help

# عرض مساعدة أمر محدد
tarqeem compile --help
tarqeem run --help
```

### الموارد

- [دليل البداية السريعة](GETTING_STARTED.md)
- [توثيق اللغة](README.md)
- [المستودع على GitHub](https://github.com/osama1998H/tarqeem)
- [الإبلاغ عن مشكلة](https://github.com/osama1998H/tarqeem/issues)

---

**ترقيم** - أول لغة برمجة عربية مُترجَمة

</div>
