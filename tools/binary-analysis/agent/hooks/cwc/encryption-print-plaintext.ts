export const address = "0x1422AE020";

export function onEnter(args: InvocationArguments) {
    let plaintext_addr = args[0];
    let plaintext_len = args[1].toUInt32();
    let plaintext = plaintext_addr.readByteArray(plaintext_len);

    if (plaintext === null) {
        console.warn("Plaintext pointer was NULL");
        return;
    }

    console.log(`cwc_encrypt called (plaintext_addr=${plaintext_addr.toString()},len=${plaintext_len})`);
    console.log(hexdump(plaintext, {offset: 0, length: plaintext_len, header: true, ansi: false}));
}