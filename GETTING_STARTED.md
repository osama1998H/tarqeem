<div dir="rtl" align="right">

# دليل البداية السريعة | Getting Started

ابدأ مع ترقيم في 5 دقائق!

</div>

## Prerequisites | المتطلبات

- [Rust](https://rustup.rs/) (1.70+)
- Git

## Installation | التثبيت

```bash
# Clone the repository
git clone https://github.com/osama1998H/tarqeem.git
cd tarqeem

# Build
cargo build --release

# Verify installation
cargo run -- --version
```

## Hello World | مرحباً بالعالم

Create a file named `مرحبا.ترقيم` (or `hello.trq`):

```tarqeem
اطبع("مرحباً بالعالم!");
```

## Run Your Program | تشغيل البرنامج

```bash
# Run directly
cargo run -- run مرحبا.ترقيم

# Or compile first
cargo run -- compile مرحبا.ترقيم -o مرحبا
./مرحبا
```

## Quick Examples | أمثلة سريعة

### Variables | المتغيرات

```tarqeem
متغير اسم = "أحمد";
ثابت عمر = 25;
اطبع(اسم);
```

### Functions | الدوال

```tarqeem
دالة جمع(أ: عدد، ب: عدد) -> عدد {
    أرجع أ + ب;
}

اطبع(جمع(5، 3));  // 8
```

### Conditionals | الشروط

```tarqeem
متغير س = 10;

إذا (س > 5) {
    اطبع("أكبر من خمسة");
} وإلا {
    اطبع("خمسة أو أقل");
}
```

## Common Commands | الأوامر الشائعة

| Command | Description |
|---------|-------------|
| `cargo run -- run file.trq` | Run a program |
| `cargo run -- compile file.trq` | Compile to binary |
| `cargo run -- check file.trq` | Check for errors |
| `cargo run -- repl` | Start interactive mode |

## Next Steps | الخطوات التالية

- See [examples/](examples/) for more code samples
- Read [README.md](README.md) for full language reference
- Check [ARCHITECTURE.md](ARCHITECTURE.md) for technical details
