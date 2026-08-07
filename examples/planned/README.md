# Planned Examples / أمثلة مخططة

These examples demonstrate features that are planned but not yet fully implemented in the Tarqeem compiler/interpreter.

## Examples in this directory:

| Example | Feature | Status |
|---------|---------|--------|
| `بصمة.ترقيم` | SHA-256 hash functions (`احسب_بصمة`, `طابق_بصمة`) | Planned |
| `تعداد.ترقيم` | Enums (`تعداد`) | Parsing planned |
| `خواص.ترقيم` | Properties (`خاصية`) | Parsing planned |
| `ضغط.ترقيم` | Compression functions (`اضغط`, `فك_الضغط`) | Planned |

## Functions needed:

### Hash functions (بصمة):
- `احسب_بصمة(نص)` - Calculate SHA-256 hash
- `طابق_بصمة(بصمة١، بصمة٢)` - Compare two hashes
- `إلى_ست_عشري(نص)` - Convert to hexadecimal
- `من_ست_عشري(نص)` - Convert from hexadecimal

### Compression functions (ضغط):
- `اضغط(نص)` - Compress data with gzip
- `فك_الضغط(بيانات)` - Decompress gzip data

## Parser features needed:

### Enums (تعداد):
```tarqeem
تعداد لون {
    أحمر،
    أخضر،
    أزرق
}
```

### Properties (خاصية):
```tarqeem
صنف نقطة {
    خاصية س: عدد = 0
    خاصية ص: عدد = 0
}
```

---

Once these features are implemented, move the examples back to `examples/`.
