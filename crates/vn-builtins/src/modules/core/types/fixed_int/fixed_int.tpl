// @kinds integer
/**
 * Fixed-width two's-complement integer. Leaving the range raises
 * `integer overflow`; it never wraps, saturates or promotes to float.
 */
export declare class $T {
    static MIN_VALUE: $T;
    static MAX_VALUE: $T;
    static BITS: int;
    @throws(ParseError) static parse(s: str): $T;
    static tryParse(s: str): $T?;

    toString(): str;
    valueOf(): $T;
    toHex(): str;
    toBinary(): str;
    toOctal(): str;
    @throws(IntegerOverflow) toInt(): int;
    toFloat(): float;
    @throws(IntegerOverflow) abs(): $T; // @signed
    @throws(IntegerOverflow) negate(): $T; // @signed
    sign(): int; // @signed
    min(other: $T): $T;
    max(other: $T): $T;
    @throws(RangeError) clamp(lo: $T, hi: $T): $T;
    @throws(RangeError, IntegerOverflow) pow(exponent: int): $T;
    isEven(): bool;
    isOdd(): bool;

    countOnes(): int;
    leadingZeros(): int;
    trailingZeros(): int;
    @throws(RangeError) rotateLeft(amount: int): $T;
    @throws(RangeError) rotateRight(amount: int): $T;
    swapBytes(): $T;

    wrappingAdd(other: $T): $T;
    wrappingSub(other: $T): $T;
    wrappingMul(other: $T): $T;
    saturatingAdd(other: $T): $T;
    saturatingSub(other: $T): $T;
    saturatingMul(other: $T): $T;
    checkedAdd(other: $T): $T?;
    checkedSub(other: $T): $T?;
    checkedMul(other: $T): $T?;
    @throws(DivisionByZero, IntegerOverflow) div(other: $T): $T;
    @throws(DivisionByZero, IntegerOverflow) floorDiv(other: $T): $T;
    @throws(DivisionByZero, IntegerOverflow) ceilDiv(other: $T): $T;
    @throws(DivisionByZero, IntegerOverflow) rem(other: $T): $T;
    @throws(DivisionByZero, IntegerOverflow) mod(other: $T): $T;
}
