# Binary Analysis with Frida

Scripts to intercept functions in the game for debugging purposes.

## Requirements

- Python 3 + pip
- NodeJS

## Setup

First, install Frida by using pip to install the dependencies listed in the requirements.txt file:

```shell
> $ pip install -r requirements.txt
```

Then install the NodeJS bindings for Frida

```shell
> $ npm install
```

## Usage

Hooks for functions are created by adding new exports to `hooks/index.ts`.
For example, the CWC encryption interceptor in `cwc/encryption-print-plaintext.ts` is registered by adding:

```typescript
export * as printCwcCiphertext from 'cwc/encryption-print-plaintext';
```

A hook file must have at least an `address` value exported

```typescript
export const address = "0x14000000";
```

Additionally, it can specify the options available on `InterceptorListenerCallbacks`:

```typescript
function onEnter(args: InvocationArguments) {
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
```

The hooks can be injected by running the Python script.

```shell
> $ python3 frida_hook_functions.py DarkSoulsIII.exe
```