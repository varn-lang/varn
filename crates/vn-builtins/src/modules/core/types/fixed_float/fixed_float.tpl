// @kinds float
/**
 * IEEE 754 binary32 floating-point number. Every result is rounded to 32 bits.
 */
export declare class $T {
    static MAX_VALUE: $T;
    static LOWEST: $T;
    static MIN_POSITIVE: $T;
    static NAN: $T;
    static INFINITY: $T;
    static NEG_INFINITY: $T;
    static EPSILON: $T;
    @throws(ParseError) static parse(s: str): $T;
    static tryParse(s: str): $T?;
    isNaN(): bool;
    isFinite(): bool;
    isInfinite(): bool;

    toString(): str;
    valueOf(): $T;
    @throws(RangeError) toFixed(digits?: int): str;
    abs(): $T;
    sign(): $T;
    negate(): $T;
    min(other: $T): $T;
    max(other: $T): $T;
    @throws(RangeError) clamp(lo: $T, hi: $T): $T;
    pow(exponent: $T): $T;
    isInteger(): bool;
    floor(): $T;
    ceil(): $T;
    round(): $T;
    trunc(): $T;
    toFloat(): float;
    @throws(IntegerOverflow) toInt(): int;
}
