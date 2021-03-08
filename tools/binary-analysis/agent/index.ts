type Hook = { address: string } & InvocationListenerCallbacks;
type HookCollection = Record<string, Hook>;
import * as hooks from './hooks/index';

const hookCollection = hooks as HookCollection;
for (const hookName in hooks) {
    const hook = hookCollection[hookName];
    const pointer = hook.address.startsWith('0x') ? ptr(hook.address) : DebugSymbol.fromName(hook.address).address;

    Interceptor.attach(pointer, hook);
}