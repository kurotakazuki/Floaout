# Floaout
Floaout is the next-generation audio format.

# TODO
- error handling
- Clarify whether #[derive(Order)] is needed
- Add Functions like pow, sinh, ...

# Bubble File Format Specification

## Metadata
| Name | `Type` (Bytes) | Description |
| ------------- | ------------- | ------------- |
| Bubble | `str` (3) | "bub" means Bubble |
| Version | `u8` (1) | Version of Bubble |
| Bubble ID | `u128` (16) | Bubble ID of this file. If undefined Bubble, than value is 0. |
| Frames | `u64` (8) | Number of frames |
| Samples Per Sec | `f64` (8) | Samples per sec |
| SampleKind | `u8` (1) | `SampleKind` |
| BubbleSampleKind | `u8` (1) | `BubbleSampleKind` |
| Name Size | `u8` (1) | Name Size |
| Name | `String` | Name (UTF-8) |
| CRC | `` () | Pending |

### SampleKind
| Variant  | Description | Value (`Type`) |
| ------------- | ------------- | ------------- |
| F32LE | `f32` Little Endian | 0 (`u8`) |
| F64LE | `f64` Little Endian | 1 (`u8`) |

### BubbleSampleKind
| Variant  | Description | Value |
| ------------- | ------------- | ------------- |
| LPCM | LPCM | 0 |
| Expression | Expression | 1 |

## Each Sample
| Bubble Sample |  | `BubbleSample` |


## Bubble Sample
| Name | `Type` (Bytes) | Description |
| ------------- | ------------- | ------------- |
| Connected, Ended and Functions size | `u16` (1) | Connected, Ended and Functions size |
| Bubble's X coordinate | `Sum` | Bubble's X coordinate (X_0) |
| Space | `char` (1) | ' ' |
| Bubble's Y coordinate | `Sum` | Bubble's Y coordinate (Y_0) |
| Space | `char` (1) | ' ' |
| Bubble's Z coordinate | `Sum` | Bubble's Z coordinate (Z_0) |
| Space | `char` (1) | ' ' |
| Domain | `OrOrExpression` |  |
| Space | `char` (1) | ' ' |
| Volume | `Sum` |  |
| Space or Semicolon | `char` (1) | ' ' if there is another |
| Ending Relative Time | `u64` (8) | Number of frames at the end of function. |
| Next Starting Relative Time | `u64` (8) | Number of frames at the start of the next function. Optional (!connected && !ended) |
| Sample Data |  | Sample Data |

### Connected, Ended and Functions size
| Name | `Type` (bits) | Description |
| ------------- | ------------- | ------------- |
| Functions Size | `u16` (14)  | Functions Size is 14 bits |

| ended \ connected | 0?????????????? | 1?????????????? |
| ------------- | ------------- | ------------- |
| ?0????????????? | Stopped (NST) | Normal |
| ?1????????????? | Ended | Ended |

### Sample Data
#### LPCM
| Name | `Type` (Bytes) | Description |
| ------------- | ------------- | ------------- |
| WavSample | `f32` or `f64` (4 or 8) | depends on `SampleKind` |

#### Expression
| Name | `Type` (Bytes) | Description |
| ------------- | ------------- | ------------- |
| Expression Size | `u16` (2) | Expression Size |
| Expression | `Sum` | Expression |


### Keywords

#### Variables
| Keyword | Description |
| ------------- | ------------- |
| X | Speaker's absolute X coordinate |
| Y | Speaker's absolute Y coordinate |
| Z | Speaker's absolute Z coordinate |
| x | x = X - X_0 (X_0 is Bubble's absolute X coordinate). Speaker's relative X coordinate. |
| y | y = Y - Y_0 (Y_0 is Bubble's absolute Y coordinate). Speaker's relative Y coordinate. |
| z | z = Z - Z_0 (Z_0 is Bubble's absolute Z coordinate). Speaker's relative Z coordinate. |
| T | Number of frames starting from the file. Absolute Time (`as f64`) |
| t | Number of frames starting from the function. Relative Time (`as f64`) |
| F | Samples per sec |

##### Constants
| Keyword | Description |
| ------------- | ------------- |
| E | Euler's number |
| PI | Pi |

#### Functions
| Keyword | Description |
| ------------- | ------------- |
| sin | Sine |
| cos | Cosine |
| tan | Tangent |
| ln | The natural logarithm of the number. |
| lg | The base 2 logarithm of the number. |

#### Others
| Keyword | Description |
| ------------- | ------------- |
| f | `f????????` `f64` |

### Punctuation
| Symbol | Name |
| ------------- | ------------- |
| + | Plus |
| - | Minus |
| * | Star |
| / | Slash |
| && | AndAnd |
| || | OrOr |
| == | EqEq |
| != | Ne |
| > | Gt |
| < | Lt |
| >= | Ge |
| <= | Le |

### Delimiters
| Symbol | Name |
| ------------- | ------------- |
|   | Space |
| , | Comma |
| ; | Semicolon |
| ( ) | Parentheses |

### Syntax
```rust
// BubbleFunctions
BubbleFunctions = BubbleFunction ZeroOrMoreBubbleFunctions / f
ZeroOrMoreBubbleFunctions = SpaceAndBubbleFunction ZeroOrMoreBubbleFunctions / Semicolon

SpaceAndBubbleFunction = Space BubbleFunction / f

// BubbleFunction
// BubbleFunction = Sum Space Sum Space Sum Space OrOrExpression Space Sum
BubbleFunction = SumAndSpace BubbleFunction1 / f
BubbleFunction1 = SumAndSpace BubbleFunction2 / f
BubbleFunction2 = SumAndSpace BubbleFunction3 / f
BubbleFunction3 = OrOrExpressionAndSpace BubbleFunction4 / f
BubbleFunction4 = Sum () / f

SumAndSpace = Sum Space / f
OrOrExpressionAndSpace = OrOrExpression Space / f

// OrOr Expression
OrOrExpression = AndAndExpression OrOrExpression1 / AndAndExpression
OrOrExpression1 = OrOr OrOrExpression / f

OrOr = "||" () / f

// AndAnd Expression
AndAndExpression = ComparisonExpression AndAndExpression1 / ComparisonExpression
AndAndExpression1 = AndAnd AndAndExpression / f

AndAnd = "&&" () / f

// Comparison Expression
ComparisonExpression = Sum ComparisonExpression1 / f
ComparisonExpression1 = Comparison Sum / f

Comparison = EqEq () / Comparison1
Comparison1 = Ne () / Comparison2
Comparison2 = Ge () / Comparison3
Comparison3 = Le () / Comparison4
Comparison4 = Gt () / Comparison5
Comparison5 = Lt () / f

EqEq = "==" () / f
Ne = "!=" () / f
Ge = ">=" () / f
Le = "<=" () / f
Gt = '>' () / f
Lt = '<' () / f

// Sum
Sum = Term ZeroOrMorePlusOrMinusAndTerms / f
ZeroOrMorePlusOrMinusAndTerms = PlusOrMinusAndTerm ZeroOrMorePlusOrMinusAndTerms / ()
PlusOrMinusAndTerm = PlusOrMinus Term / f

// Term
Term = Factor ZeroOrMoreStarOrSlashAndFactors / f
ZeroOrMoreStarOrSlashAndFactors = StarOrSlashAndFactor ZeroOrMoreStarOrSlashAndFactors / ()
StarOrSlashAndFactor = StarOrSlash Factor / f

// Factor
Factor = PlusOrMinus Factor / Power

// Power
Power = Atom PowerAndFactor / Atom
PowerAndFactor = '^' Factor / f

// Atom
Atom = ExpressionInParentheses () / Atom1
Atom1 = FloatLiteral () / Atom2
Atom2 = IntegerLiteral () / Atom3
Atom3 = Function () / Atom4
Atom4 = Variable () / Atom5
Atom5 = Constant () / f

// Variable
Variable = UppercaseX () / Variable1
Variable1 = UppercaseY () / Variable2
Variable2 = UppercaseZ () / Variable3
Variable3 = LowercaseX () / Variable4
Variable4 = LowercaseY () / Variable5
Variable5 = LowercaseZ () / Variable6
Variable6 = UppercaseT () / Variable7
Variable7 = LowercaseT () / Variable8
Variable8 = UppercaseF () / f

UppercaseX = 'X' () / f
UppercaseY = 'Y' () / f
UppercaseZ = 'Z' () / f
LowercaseX = 'x' () / f
LowercaseY = 'y' () / f
LowercaseZ = 'z' () / f
UppercaseT = 'T' () / f
LowercaseT = 't' () / f
UppercaseF = 'F' () / f

// Constant
Constant = E () / Constant1
Constant1 = Pi () / f

E = 'E' () / f
Pi = "PI" () / f

// Function
Function = Sine () / Function1
Function1 = Cosine () / Function2
Function2 = Tangent () / Function3
Function3 = Ln () / Function4
Function4 = Lg () / f

Sine = "sin" Factor / f
Cosine = "cos" Factor / f
Tangent = "tan" Factor / f
Ln = "ln" Factor / f
Lg = "lg" Factor / f

// Delimiters
ExpressionInParentheses = '(' ExpressionAndClose / f
ExpressionAndClose = Sum ')' / f

// Integer
IntegerLiteral = DecLiteral () / f

// Float
FloatLiteral = DecLiteral PointAndDecLiteral / BytesF64Literal
PointAndDecLiteral = '.' DecLiteral / f

BytesF64Literal = 'f' ???????? / f

// Dec
DecLiteral = DecDigit ZeroOrMoreDecDigits / f
ZeroOrMoreDecDigits = DecDigit ZeroOrMoreDecDigits / ()

DecDigit = '0' () / DecDigit1
DecDigit1 = '1' () / DecDigit2
DecDigit2 = '2' () / DecDigit3
DecDigit3 = '3' () / DecDigit4
DecDigit4 = '4' () / DecDigit5
DecDigit5 = '5' () / DecDigit6
DecDigit6 = '6' () / DecDigit7
DecDigit7 = '7' () / DecDigit8
DecDigit8 = '8' () / '9'

// Others
PlusOrMinus = Plus () / PlusOrMinus1
PlusOrMinus1 = Minus () / f
Plus = '+' () / f
Minus = '-' () / f

StarOrSlash = Star () / StarOrSlash1
StarOrSlash1 = Slash () / f
Star = '*' () / f
Slash = '/' () / f

Semicolon = ';' () / f
Space = ' ' () / f
```