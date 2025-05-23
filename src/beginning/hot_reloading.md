# Code Hot Reloading

Fyrox supports code hot reloading (CHR for short), which allows you to recompile the game code while the game is running. 
This functionality significantly reduces iteration times and allows rapid prototyping. This way, Rust becomes a sort of 
"scripting" language, but with all Rust safety and performance guarantees. CHR in action looks like this:

<iframe width="560" height="315" src="https://www.youtube.com/embed/vq6P3Npydmw" title="YouTube video player" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>

## How To Use

> ⚠️ If you have an existing project from one of the previous versions of the engine, the best way to add support for
> CHR is to re-generate the entire project and copy all the assets and game code in the new project. CHR requires very
> specific project structure and a small mistake in it could lead to incorrect behavior.

CHR is quite simple to use - a project generated by the project manager or `fyrox-template` already has all that is 
needed for hot reloading. There are two ways of enabling hot reloading support—using the project manager and doing 
the same manually using console commands.

### Project Manager

The easiest way of enabling hot reloading support is to simply click on `Hot Reloading` checkbox in the project
manager and click `Edit` or `Run`:

![project manager hot reloading](project_manager_hot_reloading.png)

Note the small "fire" icon, it means that the project has this feature turned on. You can enable or disable it at any
time. 

### Console Commands

Doing the same via console commands requires some bootstrapping. At first, you need to compile your game plugin using the following 
command:

```shell
RUSTFLAGS="-C prefer-dynamic=yes" cargo build --package game_dylib --no-default-features --features="dylib-engine" --profile dev-hot-reload
```

This command will compile the engine DLL (`fyrox_dylib.dll/so`) and the plugin DLL (`game_dylib.dll/so`). Please note the
mandatory environment variable `RUSTFLAGS="-C prefer-dynamic=yes"`. It forces the compiler to link the standard library 
dynamically. It is crucial because if not set, the standard library will be duplicated in game plugin and engine,
which will lead to subtle bugs.

> ⚠️ Environment variables can be set in a different ways, depending on your OS. On Linux it simply prepends the actual
> command, on Windows it requires a [separate command](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/set_1#examples). 
> Other OSes can have their own ways of setting environment variables.

The next step is to compile the editor in CHR mode. To do that, run the following command:

```shell
RUSTFLAGS="-C prefer-dynamic=yes" cargo run --package editor --no-default-features --features="dylib" --profile dev-hot-reload
```

This command will compile the editor in CHR mode and run it. After this, all you need to do is to select build profile
in the editor to be `Debug (HR)`:

![img.png](build_profile.png)

Once that's done you can run your game by clicking on the green `Play` button. You can switch between CHR and normal mode
(static linking) at any time. Keep in mind that if you run the editor in CHR mode, it will also reload all changed plugins.

## Build Profiles

CHR uses separate build profiles: `dev-hot-reload` (no optimizations) and `release-hot-reload` (with optimizations). 
Separate build profiles allow you to quickly switch between statically linked plugins and code hot reloading. This could
be useful if you're experiencing some issues with hot reloading (see next section for more info).

## Stability

CHR is a very new and experimental feature of the engine. It is based on wildly unsafe functionality which could result
in memory corruption, subtle bugs, etc. If you experience weird behavior of your game after hot reloading, run the
game in normal (static linking) mode instead. Please report any bugs in the [issue tracker](https://github.com/FyroxEngine/Fyrox/issues) 
of the engine. CHR was tested on two relatively large games - [Fish Folly](https://github.com/mrDIMAS/FishFolly) and 
[Station Iapetus](https://github.com/mrDIMAS/StationIapetus). You can download these projects and try CHR yourself.

## Technical Details and Limitations

CHR is using the standard operating system (OS) mechanism of shared libraries (DLL for short). Pretty much any OS can load
native code into a running process dynamically from a DLL. Any dynamically loaded library can then be unloaded from the
process memory. This gives a perfect opportunity to reload game code in runtime. It may sound quite easy, but in practice, 
there are a lot of issues.

### Plugin Entities and Reloading

Plugins can supply the engine with a predefined set of entities (such as scripts, etc.). These entities are serialized into 
a memory blob before the plugin itself is unloaded. When all plugins are reloaded, this memory blob is used to restore
the state of plugin entities. That being said, pretty much all the plugin entities must be serializable (implement `Visit` trait).

### Trait Objects

Trait objects are very problematic with hot reloading, because internally trait objects contain vtable with function
pointers. These pointers can be easily invalidated if the plugin is unloaded. This applies even to engine trait objects
if they're created directly from the plugin side. The only way to bypass this issue is to use special methods from the
engine to create its trait objects. It is possible to add a lint to clippy to check for such cases (see the respective 
[issue](https://github.com/rust-lang/rust-clippy/issues/12819)).

### Dangling Objects

The current plugin system tries its best to remove all plugin entities from the engine internals before reloading plugins.
However, some objects could be overlooked by this system, which could result in crash or memory corruption. The current  
approach of preventing from having dangling objects is based on the built-in reflection system—the plugin system iterates 
across all fields of every object and checks its assembly name. If the assembly name matches the plugin's assembly name, 
then this object must be deleted before the plugin is unloaded. 

### Non-serializable Entities

Not every object can be serialized, and in this case, the current plugin system calls a special method to restore such
non-serializable entities after hot reloading. Such entities could include server connections, job queues, etc.