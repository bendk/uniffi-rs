# Decorator objects

We'll be considering the UDL for an object called `DemoRustObject`:

```webidl
interface DemoRustObject {
    void do_expensive_thing();
    [Throws=RustyError]
    void do_crashy_thing();
    void do_consequential_thing();
}
```

`DemoRustObject` is a `struct` written in Rust, and uniffi generates a class of that name in the foreign languages which forward method calls to the Rust implementation.

## Problem

Just by the names of these methods, we can see that the application might not want to deal with the `DemoRustObject` by itself, without some proper care taken while calling its methods:

* it may want to run `do_expensive_thing()` off the main thread
* it may want to catch and report errors from `do_crashy_thing()`.
* it may want to inform the rest of the application each time `do_consequential_thing()` is called.

A common pattern when using uniffied code is to run some common, but app-specific, code before or after calling the Rust code.

```kotlin
class DemoLib(
    val backgroundScope: CoroutineContext,
    val errorReporter: (e: Exception) -> Unit,
    val listener: () -> Void
) {
    val demoRustObject = new DemoRustObject()

    fun doExpensiveThing() {
        backgroundScope.launch {
            demoRustObject.doExpensiveThing()
        }
    }

    fun doCrashyThing() {
        try {
            demoRustObject.doCrashyThing()
        } catch (e: Exception) {
            errorReporter(e)
        }
    }

    fun doConsequentialThing() {
        demoRustObject.doConsequentialThing()
        listener()
    }
}
```

This causes a proliferation of boiler plate code. Worse, everytime we add a new method to the `DemoRustObject`, handwritten foreign language code needs to be written to expose it safely.

The more methods that want to do similar things, the more repetitive this gets.

We could isolate the repeated code into a decorator class with `onBackgroundThread`, `withErrorReporter` and `thenNotifyListeners` methods.

```kotlin
class MyDemoDecorator(
    val backgroundScope: CoroutineContext,
    val errorReporter: (e: Exception) -> Unit,
    val listener: () -> Void
) {
    fun onBackgroundThread(rustCall: () -> Unit) {
        backgroundScope.launch {
            rustCall()
        }
    }

    fun <T> withErrorReporter(rustCall: () -> T) =
        try {
            rustCall()
        } catch (e: Exception) {
            errorReporter(e)
        }

    fun thenNotifyListeners(rustCall: () -> T) {
        try {
            rustCall()
        } finally {
            listener()
        }
    }
}
```

Then, we can re-write the `DemoLib` as:

```kotlin
class DemoLib(
    val decorator: DemoDecorator = MyDemoDecorator()
) {
    val demoRustObject = new DemoRustObject()

    fun doExpensiveThing() = decorator.onBackgroundThread {
        demoRustObject.doExpensiveThing()
    }

    fun doCrashyThing() = decorator.withErrorReporter {
        demoRustObject.doCrashyThing()
    }

    fun doConsequentialThing() = decorator.thenNotifyListeners(this) {
        demoRustObject.doConsequentialThing()
    }
}
```

This is much better, but it's still looking a bit cut and pasty. Enter decorator objects.

## Solution

```webidl
namespace demo {}

[Decorator=DemoDecorator]
interface DemoRustObject {
    [CallsWith=on_background_thread]
    void do_expensive_thing();

    [Throws=RustyError, CallsWith=with_error_reporter]
    void do_crashy_thing();

    [CallsWith=then_notify_listeners]
    void do_consequential_thing();
}
```

With this UDL: the `[Decorator=DemoDecorator]` annotation declares that `DemoRustObject` depends on `DemoDecorator` as a decorator object.

> Decorator objects take their name from [Python's decorator methods][py-decorators].
>
> In Pythonic terms, Uniffi's decorator objects are a collection of decorator functions.
>
> Swift and Kotlin don't provide functionality to capture arbitrary `*args` and call a function with those same `*args`, so
> at this time, decorator objects aren't as powerful as Python decorators. Nevertheless, they can be still quite useful.

[py-decorators]: https://www.python.org/dev/peps/pep-0318/#on-the-name-decorator

The `DemoDecorator` class must be accesable to UniFFI.  On Kotlin this means it must be in the same package as the generated bindings.  For each name
used in a `CallsWith` attribute, DemoDecorator must have a corresponding method that will be used to decorate the original method call:
  * It inputs the original method call as a zero-argument closure.
  * It should arrange for that closure to be called somehow, not necessarily immediately.
  * It can return any type it wants -- often decoraters will transform the original return type.
  * For typed languages, it must be generic on the return value of the original method.

For example:

```kotlin
class DemoDecorator (
    val backgroundScope: CoroutineContext,
    val errorReporter: (e: Exception) -> Unit,
    val listener: () -> Void
) {
    fun <T> onBackgroundThread(rustCall: () -> T) Deferred<T>
    fun <T> withErrorReporter(rustCall: () -> T): T?
    fun <T> thenNotifyListeners(rustCall: () -> T): Unit
}
```

This changes the generated API of `DemoRustObject`:

* it adds a `DemoDecorator` argument to every constructor.
* it changes the return types and throws types for each method called with a decorator method to match the decorator method.

Now the generated Rust calls go through the app-specific decorator methods, we could more safely hand the rust object to the application to use directly.

```kotlin
val decorator = MyDemoDecorator(backgroundScope, errorReporter, listener)
val demoRustObject = DemoRustObject(decorator)
```

This is a considerable improvement! Now the decorator methods can be specified by the app, and the UDL uses them. Each time the UDL changes, the foreign language bindings keeps up.

### Re-writing the wrapper

This would happen differently than the original proposal, but I'm not sure
exactly what to suggest because I don't understand the issue so well.  If the
point is to avoid changes to the code that constructs DemoLib/DemoRustObject,
then what about one of these?

  - Making DemoRustObject an open class and DemoLib a subclass whose constructor inputs the args you want
  - A factory function that inputs the args you want and returns a DemoRustObject
