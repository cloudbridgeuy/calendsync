## Manual Setup (Tauri CLI)

If you already have an existing frontend or prefer to set it up yourself, you can use the Tauri CLI to initialize the backend for your project separately.

> Note
>
> The following example assumes you are creating a new project. If you’ve already initialized the frontend of your application, you can skip the first step.
>
> Create a new directory for your project and initialize the frontend. You can use plain HTML, CSS, and JavaScript, or any framework you prefer such as Next.js, Nuxt, Svelte, Yew, or Leptos. You just need a way of serving the app in your browser. Just as an example, this is how you would setup a simple Vite app:

```bash
mkdir tauri-app
cd tauri-app
bun create vite
```

Then, install Tauri’s CLI tool using your package manager of choice. If you are using cargo to install the Tauri CLI, you will have to install it globally.

```bash
cargo install tauri-cli --version "^2.0.0" --locked
```

Determine the URL of your frontend development server. This is the URL that Tauri will use to load your content. For example, if you are using Vite, the default URL is http://localhost:5173.

In your project directory, initialize Tauri:

```bash
cargo tauri init
```

After running the command it will display a prompt asking you for different options:

✔ What is your app name? tauri-app
✔ What should the window title be? tauri-app
✔ Where are your web assets located? ..
✔ What is the url of your dev server? http://localhost:5173
✔ What is your frontend dev command? pnpm run dev
✔ What is your frontend build command? pnpm run build

This will create a src-tauri directory in your project with the necessary Tauri configuration files.

Verify your Tauri app is working by running the development server:

```bash
cargo tauri dev
```

This command will compile the Rust code and open a window with your web content.

Congratulations! You’ve created a new Tauri project using the Tauri CLI! 🚀

## Project Structure

A Tauri project is usually made of 2 parts, a Rust project and a JavaScript project (optional), and typically the setup looks something like this:

```
.
├── package.json
├── index.html
├── src/
│ ├── main.js
├── src-tauri/
│ ├── Cargo.toml
│ ├── Cargo.lock
│ ├── build.rs
│ ├── tauri.conf.json
│ ├── src/
│ │ ├── main.rs
│ │ └── lib.rs
│ ├── icons/
│ │ ├── icon.png
│ │ ├── icon.icns
│ │ └── icon.ico
│ └── capabilities/
│ └── default.json
```

In this case, the JavaScript project is at the top level, and the Rust project is inside src-tauri/, the Rust project is a normal Cargo project with some extra files:

tauri.conf.json is the main configuration file for Tauri, it contains everything from the application identifier to dev server url, this file is also a marker for the Tauri CLI to find the Rust project, to learn more about it, see Tauri Config
capabilities/ directory is the default folder Tauri reads capability files from (in short, you need to allow commands here to use them in your JavaScript code), to learn more about it, see Security
icons/ directory is the default output directory of the tauri icon command, it’s usually referenced in tauri.conf.json > bundle > icon and used for the app’s icons
build.rs contains tauri_build::build() which is used for tauri’s build system
src/lib.rs contains the Rust code and the mobile entry point (the function marked with #[cfg_attr(mobile, tauri::mobile_entry_point)]), the reason we don’t write directly in main.rs is because we compile your app to a library in mobile builds and load them through the platform frameworks
src/main.rs is the main entry point for the desktop, and we run app_lib::run() in main to use the same entry point as mobile, so to keep it simple, don’t modify this file, modify lib.rs instead. Note that app_lib corresponds to [lib.name] in Cargo.toml.

Tauri works similar to a static web host, and the way it builds is that you would compile your JavaScript project to static files first, and then compile the Rust project that will bundle those static files in, so the JavaScript project setup is basically the same as if you were to build a static website, to learn more, see Frontend Configuration

If you want to work with Rust code only, simply remove everything else and use the src-tauri/ folder as your top level project or as a member of your Rust workspace

## Frontend Configuration

Tauri is frontend agnostic and supports most frontend frameworks out of the box. However, sometimes a framework need a bit of extra configuration to integrate with Tauri. Below is a list of frameworks with recommended configurations.

If a framework is not listed then it may work with Tauri with no additional configuration needed or it could have not been documented yet. Any contributions to add a framework that may require additional configuration are welcome to help others in the Tauri community.

### Configuration Checklist

Conceptually Tauri acts as a static web host. You need to provide Tauri with a folder containing some mix of HTML, CSS, Javascript and possibly WASM that can be served to the webview Tauri provides.

Below is a checklist of common scenarios needed to integrate a frontend with Tauri:

Use static site generation (SSG), single-page applications (SPA), or classic multi-page apps (MPA). Tauri does not natively support server based alternatives (such as SSR).
For mobile development, a development server of some kind is necessary that can host the frontend on your internal IP.
Use a proper client-server relationship between your app and your API’s (no hybrid solutions with SSR).

## Vite

Vite is a build tool that aims to provide a faster and leaner development experience for modern web projects. This guide is accurate as of Vite 5.4.8.

Checklist

- Use ../dist as frontendDist in src-tauri/tauri.conf.json.
- Use process.env.TAURI_DEV_HOST as the development server host IP when set to run on iOS physical devices.

Example configuration

Update Tauri configuration

Assuming you have the following dev and build scripts in your package.json:

```json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri"
  }
}
```

You can configure the Tauri CLI to use your Vite development server and dist folder along with the hooks to automatically run the Vite scripts:

_tauri.conf.json_

```json
{
  "build": {
    "beforeDevCommand": "bun run dev",
    "beforeBuildCommand": bun run build",
    "devUrl": "http://localhost:5173",
    "frontendDist": "../dist"
  }
}
```

### Update Vite configuration:

_vite.config.js_

```typescript
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  // prevent vite from obscuring rust errors
  clearScreen: false,
  server: {
    // make sure this port matches the devUrl port in tauri.conf.json file
    port: 5173,
    // Tauri expects a fixed port, fail if that port is not available
    strictPort: true,
    // if the host Tauri is expecting is set, use it
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,

    watch: {
      // tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
  // Env variables starting with the item of `envPrefix` will be exposed in tauri's source code through `import.meta.env`.
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    // Tauri uses Chromium on Windows and WebKit on macOS and Linux
    target:
      process.env.TAURI_ENV_PLATFORM == "windows" ? "chrome105" : "safari13",
    // don't minify for debug builds
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    // produce sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
});
```

## Process Model

Tauri employs a multi-process architecture similar to Electron or many modern web browsers. This guide explores the reasons behind the design choice and why it is key to writing secure applications.

Why Multiple Processes?
In the early days of GUI applications, it was common to use a single process to perform computation, draw the interface and react to user input. As you can probably guess, this meant that a long-running, expensive computation would leave the user interface unresponsive, or worse, a failure in one app component would bring the whole app crashing down.

It became clear that a more resilient architecture was needed, and applications began running different components in different processes. This makes much better use of modern multi-core CPUs and creates far safer applications. A crash in one component doesn’t affect the whole system anymore, as components are isolated on different processes. If a process gets into an invalid state, we can easily restart it.

We can also limit the blast radius of potential exploits by handing out only the minimum amount of permissions to each process, just enough so they can get their job done. This pattern is known as the Principle of Least Privilege, and you see it in the real world all the time. If you have a gardener coming over to trim your hedge, you give them the key to your garden. You would not give them the keys to your house; why would they need access to that? The same concept applies to computer programs. The less access we give them, the less harm they can do if they get compromised.

The Core Process
Each Tauri application has a core process, which acts as the application’s entry point and which is the only component with full access to the operating system.

The Core’s primary responsibility is to use that access to create and orchestrate application windows, system-tray menus, or notifications. Tauri implements the necessary cross-platform abstractions to make this easy. It also routes all Inter-Process Communication through the Core process, allowing you to intercept, filter, and manipulate IPC messages in one central place.

The Core process should also be responsible for managing global state, such as settings or database connections. This allows you to easily synchronize state between windows and protect your business-sensitive data from prying eyes in the Frontend.

We chose Rust to implement Tauri because of its concept of Ownership guarantees memory safety while retaining excellent performance.

Diagram
Simplified representation of the Tauri process model. A single Core process manages one or more WebView processes.
The WebView Process
The Core process doesn’t render the actual user interface (UI) itself; it spins up WebView processes that leverage WebView libraries provided by the operating system. A WebView is a browser-like environment that executes your HTML, CSS, and JavaScript.

This means that most of your techniques and tools used in traditional web development can be used to create Tauri applications. For example, many Tauri examples are written using the Svelte frontend framework and the Vite bundler.

Security best practices apply as well; for example, you must always sanitize user input, never handle secrets in the Frontend, and ideally defer as much business logic as possible to the Core process to keep your attack surface small.

Unlike other similar solutions, the WebView libraries are not included in your final executable but dynamically linked at runtime1. This makes your application significantly smaller, but it also means that you need to keep platform differences in mind, just like traditional web development.

Footnotes
Currently, Tauri uses Microsoft Edge WebView2 on Windows, WKWebView on macOS and webkitgtk on Linux. ↩

## App Size

While Tauri by default provides very small binaries it doesn’t hurt to push the limits a bit, so here are some tips and tricks for reaching optimal results.

Cargo Configuration
One of the simplest frontend agnostic size improvements you can do to your project is adding a Cargo profile to it.

Dependent on whether you use the stable or nightly Rust toolchain the options available to you differ a bit. It’s recommended you stick to the stable toolchain unless you’re an advanced user.

_src-tauri/Cargo.toml_

```toml
[profile.dev]
incremental = true # Compile your binary in smaller steps.

[profile.release]
codegen-units = 1 # Allows LLVM to perform better optimization.
lto = true # Enables link-time-optimizations.
opt-level = "s" # Prioritizes small binary size. Use `3` if you prefer speed.
panic = "abort" # Higher performance by disabling panic handlers.
strip = true # Ensures debug symbols are removed.
```

### References

> Note
>
> This is not a complete reference over all available options, merely the ones that we’d like to draw extra attention to.

incremental: Compile your binary in smaller steps.
codegen-units: Speeds up compile times at the cost of compile time optimizations.
lto: Enables link time optimizations.
opt-level: Determines the focus of the compiler. Use 3 to optimize performance, z to optimize for size, and s for something in-between.
panic: Reduce size by removing panic unwinding.
strip: Strip either symbols or debuginfo from a binary.
rpath: Assists in finding the dynamic libraries the binary requires by hard coding information into the binary.
trim-paths: Removes potentially privileged information from binaries.
rustflags: Sets Rust compiler flags on a profile by profile basis.
-Cdebuginfo=0: Whether debuginfo symbols should be included in the build.
-Zthreads=8: Increases the number of threads used during compilation.
Remove Unused Commands
In Pull Request feat: add a new option to remove unused commands, we added in a new option in the tauri config file

_tauri.conf.json_

```json
{
  "build": {
    "removeUnusedCommands": true
  }
}
```

to remove commands that’re never allowed in your capability files (ACL), so you don’t have to pay for what you don’t use

> Tip
>
> To maximize the benefit of this, only include commands that you use in the ACL instead of using defaultss
>
> Note
>
> This feature requires tauri@2.4, tauri-build@2.1, tauri-plugin@2.1 and tauri-cli@2.4
>
> Note
>
> This won’t be accounting for dynamically added ACLs at runtime so make sure to check it when using this

How does it work under the hood?
tauri-cli will communicate with tauri-build and the build script of tauri, tauri-plugin through an environment variable and let them generate a list of allowed commands from the ACL, this will then be used by the generate_handler macro to remove unused commands based on that

An internal detail is this environment variable is currently REMOVE_UNUSED_COMMANDS, and it’s set to project’s directory, usually the src-tauri directory, this is used for the build scripts to find the capability files, and although it’s not encouraged, you can still set this environment variable yourself if you can’t or don’t want to use tauri-cli to get this to work (do note that as this is an implementation detail, we don’t guarantee the stability of it)

## Inter-Process Communication

Inter-Process Communication (IPC) allows isolated processes to communicate securely and is key to building more complex applications.

Learn more about the specific IPC patterns in the following guides:

Brownfield
Isolation
Tauri uses a particular style of Inter-Process Communication called Asynchronous Message Passing, where processes exchange requests and responses serialized using some simple data representation. Message Passing should sound familiar to anyone with web development experience, as this paradigm is used for client-server communication on the internet.

Message passing is a safer technique than shared memory or direct function access because the recipient is free to reject or discard requests as it sees fit. For example, if the Tauri Core process determines a request to be malicious, it simply discards the requests and never executes the corresponding function.

In the following, we explain Tauri’s two IPC primitives - Events and Commands - in more detail.

Events
Events are fire-and-forget, one-way IPC messages that are best suited to communicate lifecycle events and state changes. Unlike Commands, Events can be emitted by both the Frontend and the Tauri Core.

Diagram
Events sent between the Core and the Webview.
Commands
Tauri also provides a foreign function interface-like abstraction on top of IPC messages1. The primary API, invoke, is similar to the browser’s fetch API and allows the Frontend to invoke Rust functions, pass arguments, and receive data.

Because this mechanism uses a JSON-RPC like protocol under the hood to serialize requests and responses, all arguments and return data must be serializable to JSON.

Diagram
IPC messages involved in a command invocation.
Footnotes
Because Commands still use message passing under the hood, they do not share the same security pitfalls as real FFI interfaces do. ↩

## Brownfield Pattern

This is the default pattern.

This is the simplest and most straightforward pattern to use Tauri with, because it tries to be as compatible as possible with existing frontend projects. In short, it tries to require nothing additional to what an existing web frontend might use inside a browser. Not everything that works in existing browser applications will work out-of-the-box.

If you are unfamiliar with Brownfield software development in general, the Brownfield Wikipedia article provides a nice summary. For Tauri, the existing software is current browser support and behavior, instead of legacy systems.

Configuration
Because the Brownfield pattern is the default pattern, it doesn’t require a configuration option to be set. To explicitly set it, you can use the app > security > pattern object in the tauri.conf.json configuration file.

```json
{
  "app": {
    "security": {
      "pattern": {
        "use": "brownfield"
      }
    }
  }
}
```

There are no additional configuration options for the brownfield pattern.

## Isolation Pattern

The Isolation pattern is a way to intercept and modify Tauri API messages sent by the frontend before they get to Tauri Core, all with JavaScript. The secure JavaScript code that is injected by the Isolation pattern is referred to as the Isolation application.

Why
The Isolation pattern’s purpose is to provide a mechanism for developers to help protect their application from unwanted or malicious frontend calls to Tauri Core. The need for the Isolation pattern rose out of threats coming from untrusted content running on the frontend, a common case for applications with many dependencies. See Security: Threat Models for a list of many sources of threats that an application may see.

The largest threat model described above that the Isolation pattern was designed in mind was Development Threats. Not only do many frontend build-time tools consist of many dozen (or hundreds) of often deeply-nested dependencies, but a complex application may also have a large amount of (also often deeply-nested) dependencies that are bundled into the final output.

When
Tauri highly recommends using the isolation pattern whenever it can be used. Because the Isolation application intercepts all messages from the frontend, it can always be used.

Tauri also strongly suggests locking down your application whenever you use external Tauri APIs. As the developer, you can utilize the secure Isolation application to try and verify IPC inputs, to make sure they are within some expected parameters. For example, you may want to check that a call to read or write a file is not trying to access a path outside your application’s expected locations. Another example is making sure that a Tauri API HTTP fetch call is only setting the Origin header to what your application expects it to be.

That said, it intercepts all messages from the frontend, so it will even work with always-on APIs such as Events. Since some events may cause your own rust code to perform actions, the same sort of validation techniques can be used with them.

How
The Isolation pattern is all about injecting a secure application in between your frontend and Tauri Core to intercept and modify incoming IPC messages. It does this by using the sandboxing feature of <iframe>s to run the JavaScript securely alongside the main frontend application. Tauri enforces the Isolation pattern while loading the page, forcing all IPC calls to Tauri Core to instead be routed through the sandboxed Isolation application first. Once the message is ready to be passed to Tauri Core, it is encrypted using the browser’s SubtleCrypto implementation and passed back to the main frontend application. Once there, it is directly passed to Tauri Core, where it is then decrypted and read like normal.

To ensure that someone cannot manually read the keys for a specific version of your application and use that to modify the messages after being encrypted, new keys are generated each time your application is run.

Approximate Steps of an IPC Message
To make it easier to follow, here’s an ordered list with the approximate steps an IPC message will go through when being sent to Tauri Core with the Isolation pattern:

Tauri’s IPC handler receives a message
IPC handler -> Isolation application
[sandbox] Isolation application hook runs and potentially modifies the message
[sandbox] Message is encrypted with AES-GCM using a runtime-generated key
[encrypted] Isolation application -> IPC handler
[encrypted] IPC handler -> Tauri Core
Note: Arrows (->) indicate message passing.

Performance Implications
Because encryption of the message does occur, there are additional overhead costs compared to the Brownfield pattern, even if the secure Isolation application doesn’t do anything. Aside from performance-sensitive applications (who likely have a carefully-maintained and small set of dependencies, to keep the performance adequate), most applications should not notice the runtime costs of encrypting/decrypting the IPC messages, as they are relatively small and AES-GCM is relatively fast. If you are unfamiliar with AES-GCM, all that is relevant in this context is that it’s the only authenticated mode algorithm included in SubtleCrypto and that you probably already use it every day under the hood with TLS.

There is also a cryptographically secure key generated once each time the Tauri application is started. It is not generally noticeable if the system already has enough entropy to immediately return enough random numbers, which is extremely common for desktop environments. If running in a headless environment to perform some integration testing with WebDriver then you may want to install some sort of entropy-generating service such as haveged if your operating system does not have one included. Linux 5.6 (March 2020) now includes entropy generation using speculative execution.

Limitations
There are a few limitations in the Isolation pattern that arose out of platform inconsistencies. The most significant limitation is due to external files not loading correctly inside sandboxed <iframes> on Windows. Because of this, we have implemented a simple script inlining step during build time that takes the content of scripts relative to the Isolation application and injects them inline. This means that typical bundling or simple including of files like <script src="index.js"></script> still works properly, but newer mechanisms such as ES Modules will not successfully load.

Recommendations
Because the point of the Isolation application is to protect against Development Threats, we highly recommend keeping your Isolation application as simple as possible. Not only should you strive to keep dependencies of your isolation application minimal, but you should also consider keeping its required build steps minimal. This would allow you to not need to worry about supply chain attacks against your Isolation application on top of your frontend application.

Creating the Isolation Application
In this example, we will make a small hello-world style Isolation application and hook it up to an imaginary existing Tauri application. It will do no verification of the messages passing through it, only print the contents to the WebView console.

For the purposes of this example, let’s imagine we are in the same directory as tauri.conf.json. The existing Tauri application has its frontendDist set to ../dist.

../dist-isolation/index.html:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <title>Isolation Secure Script</title>
  </head>
  <body>
    <script src="index.js"></script>
  </body>
</html>
```

../dist-isolation/index.js:

```javascript
window.__TAURI_ISOLATION_HOOK__ = (payload) => {
  // let's not verify or modify anything, just print the content from the hook
  console.log("hook", payload);
  return payload;
};
```

Now, all we need to do is set up our tauri.conf.json configuration to use the Isolation pattern, and have just bootstrapped to the Isolation pattern from the Brownfield pattern.

Configuration
Let’s assume that our main frontend frontendDist is set to ../dist. We also output our Isolation application to ../dist-isolation.

```json
{
  "build": {
    "frontendDist": "../dist"
  },
  "app": {
    "security": {
      "pattern": {
        "use": "isolation",
        "options": {
          "dir": "../dist-isolation"
        }
      }
    }
  }
}
```

## Develop

Now that you have everything set up, you are ready to run your application using Tauri.

If you are using an UI framework or JavaScript bundler you likely have access to a development server that will speed up your development process, so if you haven’t configured your app’s dev URL and script that starts it, you can do so via the devUrl and beforeDevCommand config values:

tauri.conf.json
{
"build": {
"devUrl": "http://localhost:3000",
"beforeDevCommand": "npm run dev"
}
}

Note

Every framework has its own development tooling. It is outside of the scope of this document to cover them all or stay up to date.

Please refer to your framework’s documentation to learn more and determine the correct values to be configured.

Otherwise if you are not using a UI framework or module bundler you can point Tauri to your frontend source code and the Tauri CLI will start a development server for you:

tauri.conf.json
{
"build": {
"frontendDist": "./src"
}
}

Note that in this example the src folder must include a index.html file along any other assets loaded by your frontend.

Plain/Vanilla Dev Server Security

The built-in Tauri development server does not support mutual authentication or encryption. You should never use it for development on untrusted networks. See the development server security considerations for a more detailed explanation.

Developing Your Desktop Application
To develop your application for desktop, run the tauri dev command.

npm
yarn
pnpm
deno
bun
cargo
cargo tauri dev

The first time you run this command, the Rust package manager may need several minutes to download and build all the required packages. Since they are cached, subsequent builds are much faster, as only your code needs rebuilding.

Once Rust has finished building, the webview opens, displaying your web app. You can make changes to your web app, and if your tooling supports it, the webview should update automatically, just like a browser.

Opening the Web Inspector
You can open the Web Inspector to debug your application by performing a right-click on the webview and clicking “Inspect” or using the Ctrl + Shift + I shortcut on Windows and Linux or Cmd + Option + I shortcut on macOS.

Developing your Mobile Application
Developing for mobile is similar to how desktop development works, but you must run tauri android dev or tauri ios dev instead:

npm
yarn
pnpm
deno
bun
cargo
cargo tauri [android|ios] dev

The first time you run this command, the Rust package manager may need several minutes to download and build all the required packages. Since they are cached, subsequent builds are much faster, as only your code needs rebuilding.

Development Server
The development server on mobile works similarly to the desktop one, but if you are trying to run on a physical iOS device, you must configure it to listen to a particular address provided by the Tauri CLI, defined in the TAURI_DEV_HOST environment variable. This address is either a public network address (which is the default behavior) or the actual iOS device TUN address - which is more secure, but currently needs Xcode to connect to the device.

To use the iOS device’s address you must open Xcode before running the dev command and ensure your device is connected via network in the Window > Devices and Simulators menu. Then you must run tauri ios dev --force-ip-prompt to select the iOS device address (a IPv6 address ending with ::2).

To make your development server listen on the correct host to be accessible by the iOS device you must tweak its configuration to use the TAURI_DEV_HOST value if it has been provided. Here is an example configuration for Vite:

import { defineConfig } from 'vite';

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig({
clearScreen: false,
server: {
host: host || false,
port: 1420,
strictPort: true,
hmr: host
? {
protocol: 'ws',
host,
port: 1421,
}
: undefined,
},
});

Check your framework’s setup guide for more information.

Note

Projects created with create-tauri-app configures your development server for mobile dev out of the box.

Device Selection
By default the mobile dev command tries to run your application in a connected device, and fallbacks to prompting you to select a simulator to use. To define the run target upfront you can provide the device or simulator name as argument:

npm
yarn
pnpm
deno
bun
cargo
cargo tauri ios dev 'iPhone 15'

Using Xcode or Android Studio
Alternatively you can choose to use Xcode or Android Studio to develop your application. This can help you troubleshoot some development issues by using the IDE instead of the command line tools. To open the mobile IDE instead of running on a connected device or simulator, use the --open flag:

npm
yarn
pnpm
deno
bun
cargo
cargo tauri [android|ios] dev --open

Note

If you intend on running the application on a physical iOS device you must also provide the --host argument and your development server must use the process.env.TAURI_DEV_HOST value as host. See your framework’s setup guide for more information.

npm
yarn
pnpm
deno
bun
cargo
cargo tauri [android|ios] dev --open --host

Caution

To use Xcode or Android Studio the Tauri CLI process must be running and cannot be killed. It is recommended to use the tauri [android|ios] dev --open command and keep the process alive until you close the IDE.

Opening the Web Inspector
iOS

Safari must be used to access the Web Inspector for your iOS application.

Open the Safari on your Mac machine, choose Safari > Settings in the menu bar, click Advanced, then select Show features for web developers.

If you are running on a physical device you must enable Web Inspector in Settings > Safari > Advanced.

After following all steps you should see a Develop menu in Safari, where you will find the connected devices and applications to inspect. Select your device or simulator and click on localhost to open the Safari Developer Tools window.

Android

The inspector is enabled by default for Android emulators, but you must enable it for physical devices. Connect your Android device to the computer, open the Settings app in the Android device, select About, scroll to Build Number and tap that 7 times. This will enable Developer Mode for your Android device and the Developer Options settings.

To enable application debugging on your device you must enter the Developer Options settings, toggle on the developer options switch and enable USB Debugging.

Note

Each Android distribution has its own way to enable the Developer Mode, please check your manufacturer’s documentation for more information.

The Web Inspector for Android is powered by Google Chrome’s DevTools and can be accessed by navigating to chrome://inspect in the Chrome browser on your computer. Your device or emulator should appear in the remote devices list if your Android application is running, and you can open the developer tools by clicking inspect on the entry matching your device.

Troubleshooting
Error running build script on Xcode
Tauri hooks into the iOS Xcode project by creating a build phase that executes the Tauri CLI to compile the Rust source as a library that is loaded at runtime. The build phase is executed on the Xcode process context, so it might not be able to use shell modifications such as PATH additions, so be careful when using tools such as Node.js version managers which may not be compatible.

Network permission prompt on first iOS app execution
On the first time you execute tauri ios dev you might see iOS prompting you for permission to find and connect to devices on your local network. This permission is required because to access your development server from an iOS device, we must expose it in the local network. To run your app in your device you must click Allow and restart your application.

Reacting to Source Code Changes
Similarly to how your webview reflects changes in real time, Tauri watches your Rust files for changes so when you modify any of them your application is automatically rebuilt and restarted.

You can disable this behavior by using the --no-watch flag on the tauri dev command.

To restrict the files that are watched for changes you can create a .taurignore file in the src-tauri folder. This file works just like a regular Git ignore file, so you can ignore any folder or file:

build/
src/generated/\*.rs
deny.toml

Using the Browser DevTools
Tauri’s APIs only work in your app window, so once you start using them you won’t be able to open your frontend in your system’s browser anymore.

If you prefer using your browser’s developer tooling, you must configure tauri-invoke-http to bridge Tauri API calls through a HTTP server.

Source Control
In your project repository, you SHOULD commit the src-tauri/Cargo.lock along with the src-tauri/Cargo.toml to git because Cargo uses the lockfile to provide deterministic builds. As a result, it is recommended that all applications check in their Cargo.lock. You SHOULD NOT commit the src-tauri/target folder or any of its contents.

## Configuration Files

Since Tauri is a toolkit for building applications there can be many files to configure project settings. Some common files that you may run across are tauri.conf.json, package.json and Cargo.toml. We briefly explain each on this page to help point you in the right direction for which files to modify.

Tauri Config
The Tauri configuration is used to define the source of your Web app, describe your application’s metadata, configure bundles, set plugin configurations, modify runtime behavior by configuring windows, tray icons, menus and more.

This file is used by the Tauri runtime and the Tauri CLI. You can define build settings (such as the command run before tauri build or tauri dev kicks in), set the name and version of your app, control the Tauri runtime, and configure plugins.

Tip

You can find all of the options in the configuration reference.

Supported Formats
The default Tauri config format is JSON. The JSON5 or TOML format can be enabled by adding the config-json5 or config-toml feature flag (respectively) to the tauri and tauri-build dependencies in Cargo.toml.

Cargo.toml
[build-dependencies]
tauri-build = { version = "2.0.0", features = [ "config-json5" ] }

[dependencies]
tauri = { version = "2.0.0", features = [ "config-json5" ] }

The structure and values are the same across all formats, however, the formatting should be consistent with the respective file’s format:

tauri.conf.json
{
build: {
devUrl: 'http://localhost:3000',
// start the dev server
beforeDevCommand: 'npm run dev',
},
bundle: {
active: true,
icon: ['icons/app.png'],
},
app: {
windows: [
{
title: 'MyApp',
},
],
},
plugins: {
updater: {
pubkey: 'updater pub key',
endpoints: ['https://my.app.updater/{{target}}/{{current_version}}'],
},
},
}

Tauri.toml
[build]
dev-url = "http://localhost:3000"

# start the dev server

before-dev-command = "npm run dev"

[bundle]
active = true
icon = ["icons/app.png"]

[[app.windows]]
title = "MyApp"

[plugins.updater]
pubkey = "updater pub key"
endpoints = ["https://my.app.updater/{{target}}/{{current_version}}"]

Note that JSON5 and TOML supports comments, and TOML can use kebab-case for config names which are more idiomatic. Field names are case-sensitive in all 3 formats.

Platform-specific Configuration
In addition to the default configuration file, Tauri can read a platform-specific configuration from:

tauri.linux.conf.json or Tauri.linux.toml for Linux
tauri.windows.conf.json or Tauri.windows.toml for Windows
tauri.macos.conf.json or Tauri.macos.toml for macOS
tauri.android.conf.json or Tauri.android.toml for Android
tauri.ios.conf.json or Tauri.ios.toml for iOS
The platform-specific configuration file gets merged with the main configuration object following the JSON Merge Patch (RFC 7396) specification.

For example, given the following base tauri.conf.json:

tauri.conf.json
{
"productName": "MyApp",
"bundle": {
"resources": ["./resources"]
},
"plugins": {
"deep-link": {}
}
}

And the given tauri.linux.conf.json:

tauri.linux.conf.json
{
"productName": "my-app",
"bundle": {
"resources": ["./linux-assets"]
},
"plugins": {
"cli": {
"description": "My app",
"subcommands": {
"update": {}
}
},
"deep-link": {}
}
}

The resolved configuration for Linux would be the following object:

{
"productName": "my-app",
"bundle": {
"resources": ["./linux-assets"]
},
"plugins": {
"cli": {
"description": "My app",
"subcommands": {
"update": {}
}
},
"deep-link": {}
}
}

Additionally you can provide a configuration to be merged via the CLI, see the following section for more information.

Extending the Configuration
The Tauri CLI allows you to extend the Tauri configuration when running one of the dev, android dev, ios dev, build, android build, ios build or bundle commands. The configuration extension can be provided by the --config argument either as a raw JSON string or as a path to a JSON file. Tauri uses the JSON Merge Patch (RFC 7396) specification to merge the provided configuration value with the originally resolved configuration object.

This mechanism can be used to define multiple flavours of your application or have more flexibility when configuring your application bundles.

For instance to distribute a completely isolated beta application you can use this feature to configure a separate application name and identifier:

src-tauri/tauri.beta.conf.json
{
"productName": "My App Beta",
"identifier": "com.myorg.myappbeta"
}

And to distribute this separate beta app you provide this configuration file when building it:

npm
yarn
pnpm
deno
bun
cargo
cargo tauri build --config src-tauri/tauri.beta.conf.json

Cargo.toml
Cargo’s manifest file is used to declare Rust crates your app depends on, metadata about your app, and other Rust-related features. If you do not intend to do backend development using Rust for your app then you may not be modifying it much, but it’s important to know that it exists and what it does.

Below is an example of a barebones Cargo.toml file for a Tauri project:

Cargo.toml
[package]
name = "app"
version = "0.1.0"
description = "A Tauri App"
authors = ["you"]
license = ""
repository = ""
default-run = "app"
edition = "2021"
rust-version = "1.57"

[build-dependencies]
tauri-build = { version = "2.0.0" }

[dependencies]
serde_json = "1.0"
serde = { version = "1.0", features = ["derive"] }
tauri = { version = "2.0.0", features = [ ] }

The most important parts to take note of are the tauri-build and tauri dependencies. Generally, they must both be on the same latest minor versions as the Tauri CLI, but this is not strictly required. If you encounter issues while trying to run your app you should check that any Tauri versions (tauri and tauri-cli) are on the latest versions for their respective minor releases.

Cargo version numbers use Semantic Versioning. Running cargo update in the src-tauri folder will pull the latest available Semver-compatible versions of all dependencies. For example, if you specify 2.0.0 as the version for tauri-build, Cargo will detect and download version 2.0.0.0 because it is the latest Semver-compatible version available. Tauri will update the major version number whenever a breaking change is introduced, meaning you should always be capable of safely upgrading to the latest minor and patch versions without fear of your code breaking.

If you want to use a specific crate version you can use exact versions instead by prepending = to the version number of the dependency:

tauri-build = { version = "=2.0.0" }

An additional thing to take note of is the features=[] portion of the tauri dependency. Running tauri dev and tauri build will automatically manage which features need to be enabled in your project based on the your Tauri configuration. For more information about tauri feature flags see the documentation.

When you build your application a Cargo.lock file is produced. This file is used primarily for ensuring that the same dependencies are used across machines during development (similar to yarn.lock, pnpm-lock.yaml or package-lock.json in Node.js). It is recommended to commit this file to your source repository so you get consistent builds.

To learn more about the Cargo manifest file please refer to the official documentation.

package.json
This is the package file used by Node.js. If the frontend of your Tauri app is developed using Node.js-based technologies (such as npm, yarn, or pnpm) this file is used to configure the frontend dependencies and scripts.

An example of a barebones package.json file for a Tauri project might look a little something like this:

package.json
{
"scripts": {
"dev": "command to start your app development mode",
"build": "command to build your app frontend",
"tauri": "tauri"
},
"dependencies": {
"@tauri-apps/api": "^2.0.0.0",
"@tauri-apps/cli": "^2.0.0.0"
}
}

It’s common to use the "scripts" section to store the commands used to launch and build the frontend used by your Tauri application. The above package.json file specifies the dev command that you can run using yarn dev or npm run dev to start the frontend framework and the build command that you can run using yarn build or npm run build to build your frontend’s Web assets to be added by Tauri in production. The most convenient way to use these scripts is to hook them with the Tauri CLI via the Tauri configuration’s beforeDevCommand and beforeBuildCommand hooks:

tauri.conf.json
{
"build": {
"beforeDevCommand": "yarn dev",
"beforeBuildCommand": "yarn build"
}
}

Note

The "tauri" script is only needed when using npm

The dependencies object specifies which dependencies Node.js should download when you run either yarn, pnpm install or npm install (in this case the Tauri CLI and API).

In addition to the package.json file you may see either a yarn.lock, pnpm-lock.yaml or package-lock.json file. These files assist in ensuring that when you download the dependencies later you’ll get the exact same versions that you have used during development (similar to Cargo.lock in Rust).

To learn more about the package.json file format please refer to the official documentation.

## Calling Rust from the Frontend

This document includes guides on how to communicate with your Rust code from your application frontend. To see how to communicate with your frontend from your Rust code, see Calling the Frontend from Rust.

Tauri provides a command primitive for reaching Rust functions with type safety, along with an event system that is more dynamic.

Commands
Tauri provides a simple yet powerful command system for calling Rust functions from your web app. Commands can accept arguments and return values. They can also return errors and be async.

Basic Example
Commands can be defined in your src-tauri/src/lib.rs file. To create a command, just add a function and annotate it with #[tauri::command]:

src-tauri/src/lib.rs #[tauri::command]
fn my_custom_command() {
println!("I was invoked from JavaScript!");
}

Note

Command names must be unique.

Note

Commands defined in the lib.rs file cannot be marked as pub due to a limitation in the glue code generation. You will see an error like this if you mark it as a public function:

error[E0255]: the name `__cmd__command_name` is defined multiple times
--> src/lib.rs:28:8
|
27 | #[tauri::command]
| ----------------- previous definition of the macro `__cmd__command_name` here
28 | pub fn x() {}
| ^ `__cmd__command_name` reimported here
|
= note: `__cmd__command_name` must be defined only once in the macro namespace of this module

You will have to provide a list of your commands to the builder function like so:

src-tauri/src/lib.rs #[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
tauri::Builder::default()
.invoke_handler(tauri::generate_handler![my_custom_command])
.run(tauri::generate_context!())
.expect("error while running tauri application");
}

Now, you can invoke the command from your JavaScript code:

// When using the Tauri API npm package:
import { invoke } from '@tauri-apps/api/core';

// When using the Tauri global script (if not using the npm package)
// Be sure to set `app.withGlobalTauri` in `tauri.conf.json` to true
const invoke = window.**TAURI**.core.invoke;

// Invoke the command
invoke('my_custom_command');

Defining Commands in a Separate Module
If your application defines a lot of components or if they can be grouped, you can define commands in a separate module instead of bloating the lib.rs file.

As an example let’s define a command in the src-tauri/src/commands.rs file:

src-tauri/src/commands.rs #[tauri::command]
pub fn my_custom_command() {
println!("I was invoked from JavaScript!");
}

Note

When defining commands in a separate module they should be marked as pub.

Note

The command name is not scoped to the module so they must be unique even between modules.

In the lib.rs file, define the module and provide the list of your commands accordingly;

src-tauri/src/lib.rs
mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
tauri::Builder::default()
.invoke_handler(tauri::generate_handler![commands::my_custom_command])
.run(tauri::generate_context!())
.expect("error while running tauri application");
}

Note the commands:: prefix in the command list, which denotes the full path to the command function.

The command name in this example is my_custom_command so you can still call it by executing invoke("my_custom_command") in your frontend, the commands:: prefix is ignored.

WASM
When using a Rust frontend to call invoke() without arguments, you will need to adapt your frontend code as below. The reason is that Rust doesn’t support optional arguments.

#[wasm_bindgen]
extern "C" {
// invoke without arguments
#[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
async fn invoke_without_args(cmd: &str) -> JsValue;

    // invoke with arguments (default)
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;

    // They need to have different names!

}

Passing Arguments
Your command handlers can take arguments:

#[tauri::command]
fn my_custom_command(invoke_message: String) {
println!("I was invoked from JavaScript, with this message: {}", invoke_message);
}

Arguments should be passed as a JSON object with camelCase keys:

invoke('my_custom_command', { invokeMessage: 'Hello!' });

Note

You can use snake_case for the arguments with the rename_all attribute:

#[tauri::command(rename_all = "snake_case")]
fn my_custom_command(invoke_message: String) {}

The corresponding JavaScript:

invoke('my_custom_command', { invoke_message: 'Hello!' });

Arguments can be of any type, as long as they implement serde::Deserialize.

Returning Data
Command handlers can return data as well:

#[tauri::command]
fn my_custom_command() -> String {
"Hello from Rust!".into()
}

The invoke function returns a promise that resolves with the returned value:

invoke('my_custom_command').then((message) => console.log(message));

Returned data can be of any type, as long as it implements serde::Serialize.

Returning Array Buffers
Return values that implements serde::Serialize are serialized to JSON when the response is sent to the frontend. This can slow down your application if you try to return a large data such as a file or a download HTTP response. To return array buffers in an optimized way, use tauri::ipc::Response:

use tauri::ipc::Response; #[tauri::command]
fn read_file() -> Response {
let data = std::fs::read("/path/to/file").unwrap();
tauri::ipc::Response::new(data)
}

Error Handling
If your handler could fail and needs to be able to return an error, have the function return a Result:

#[tauri::command]
fn login(user: String, password: String) -> Result<String, String> {
if user == "tauri" && password == "tauri" {
// resolve
Ok("logged_in".to_string())
} else {
// reject
Err("invalid credentials".to_string())
}
}

If the command returns an error, the promise will reject, otherwise, it resolves:

invoke('login', { user: 'tauri', password: '0j4rijw8=' })
.then((message) => console.log(message))
.catch((error) => console.error(error));

As mentioned above, everything returned from commands must implement serde::Serialize, including errors. This can be problematic if you’re working with error types from Rust’s std library or external crates as most error types do not implement it. In simple scenarios you can use map_err to convert these errors to String:

#[tauri::command]
fn my_custom_command() -> Result<(), String> {
std::fs::File::open("path/to/file").map_err(|err| err.to_string())?;
// Return `null` on success
Ok(())
}

Since this is not very idiomatic you may want to create your own error type which implements serde::Serialize. In the following example, we use the thiserror crate to help create the error type. It allows you to turn enums into error types by deriving the thiserror::Error trait. You can consult its documentation for more details.

// create the error type that represents all errors possible in our program #[derive(Debug, thiserror::Error)]
enum Error { #[error(transparent)]
Io(#[from] std::io::Error)
}

// we must manually implement serde::Serialize
impl serde::Serialize for Error {
fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
where
S: serde::ser::Serializer,
{
serializer.serialize_str(self.to_string().as_ref())
}
}

#[tauri::command]
fn my_custom_command() -> Result<(), Error> {
// This will return an error
std::fs::File::open("path/that/does/not/exist")?;
// Return `null` on success
Ok(())
}

A custom error type has the advantage of making all possible errors explicit so readers can quickly identify what errors can happen. This saves other people (and yourself) enormous amounts of time when reviewing and refactoring code later.
It also gives you full control over the way your error type gets serialized. In the above example, we simply returned the error message as a string, but you could assign each error a code so you could more easily map it to a similar looking TypeScript error enum for example:

#[derive(Debug, thiserror::Error)]
enum Error { #[error(transparent)]
Io(#[from] std::io::Error), #[error("failed to parse as string: {0}")]
Utf8(#[from] std::str::Utf8Error),
}

#[derive(serde::Serialize)] #[serde(tag = "kind", content = "message")] #[serde(rename_all = "camelCase")]
enum ErrorKind {
Io(String),
Utf8(String),
}

impl serde::Serialize for Error {
fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
where
S: serde::ser::Serializer,
{
let error*message = self.to_string();
let error_kind = match self {
Self::Io(*) => ErrorKind::Io(error*message),
Self::Utf8(*) => ErrorKind::Utf8(error_message),
};
error_kind.serialize(serializer)
}
}

#[tauri::command]
fn read() -> Result<Vec<u8>, Error> {
let data = std::fs::read("/path/to/file")?;
Ok(data)
}

In your frontend you now get a { kind: 'io' | 'utf8', message: string } error object:

type ErrorKind = {
kind: 'io' | 'utf8';
message: string;
};

invoke('read').catch((e: ErrorKind) => {});

Async Commands
Asynchronous commands are preferred in Tauri to perform heavy work in a manner that doesn’t result in UI freezes or slowdowns.

Note

Async commands are executed on a separate async task using async_runtime::spawn. Commands without the async keyword are executed on the main thread unless defined with #[tauri::command(async)].

If your command needs to run asynchronously, simply declare it as async.

Caution

You need to be careful when creating asynchronous functions using Tauri. Currently, you cannot simply include borrowed arguments in the signature of an asynchronous function. Some common examples of types like this are &str and State<'\_, Data>. This limitation is tracked here: https://github.com/tauri-apps/tauri/issues/2533 and workarounds are shown below.

When working with borrowed types, you have to make additional changes. These are your two main options:

Option 1: Convert the type, such as &str to a similar type that is not borrowed, such as String. This may not work for all types, for example State<'\_, Data>.

Example:

// Declare the async function using String instead of &str, as &str is borrowed and thus unsupported #[tauri::command]
async fn my_custom_command(value: String) -> String {
// Call another async function and wait for it to finish
some_async_function().await;
value
}

Option 2: Wrap the return type in a Result. This one is a bit harder to implement, but works for all types.

Use the return type Result<a, b>, replacing a with the type you wish to return, or () if you wish to return null, and replacing b with an error type to return if something goes wrong, or () if you wish to have no optional error returned. For example:

Result<String, ()> to return a String, and no error.
Result<(), ()> to return null.
Result<bool, Error> to return a boolean or an error as shown in the Error Handling section above.
Example:

// Return a Result<String, ()> to bypass the borrowing issue #[tauri::command]
async fn my_custom_command(value: &str) -> Result<String, ()> {
// Call another async function and wait for it to finish
some_async_function().await;
// Note that the return value must be wrapped in `Ok()` now.
Ok(format!(value))
}

Invoking from JavaScript
Since invoking the command from JavaScript already returns a promise, it works just like any other command:

invoke('my_custom_command', { value: 'Hello, Async!' }).then(() =>
console.log('Completed!')
);

Channels
The Tauri channel is the recommended mechanism for streaming data such as streamed HTTP responses to the frontend. The following example reads a file and notifies the frontend of the progress in chunks of 4096 bytes:

use tokio::io::AsyncReadExt;

#[tauri::command]
async fn load_image(path: std::path::PathBuf, reader: tauri::ipc::Channel<&[u8]>) {
// for simplicity this example does not include error handling
let mut file = tokio::fs::File::open(path).await.unwrap();

let mut chunk = vec![0; 4096];

loop {
let len = file.read(&mut chunk).await.unwrap();
if len == 0 {
// Length of zero means end of file.
break;
}
reader.send(&chunk).unwrap();
}
}

See the channels documentation for more information.

Accessing the WebviewWindow in Commands
Commands can access the WebviewWindow instance that invoked the message:

src-tauri/src/lib.rs #[tauri::command]
async fn my_custom_command(webview_window: tauri::WebviewWindow) {
println!("WebviewWindow: {}", webview_window.label());
}

Accessing an AppHandle in Commands
Commands can access an AppHandle instance:

src-tauri/src/lib.rs #[tauri::command]
async fn my_custom_command(app_handle: tauri::AppHandle) {
let app_dir = app_handle.path().app_dir();
use tauri::GlobalShortcutManager;
app_handle.global_shortcut_manager().register("CTRL + U", move || {});
}

Tip

AppHandle and WebviewWindow both take a generic parameter R: Runtime, when the wry feature is enabled in tauri (which is enabled by default), we default the generic to the Wry runtime so you can use it directly, but if you want to use a different runtime, for example the mock runtime, you need to write your functions like this

src-tauri/src/lib.rs
use tauri::{AppHandle, GlobalShortcutManager, Runtime, WebviewWindow};

#[tauri::command]
async fn my_custom_command<R: Runtime>(app_handle: AppHandle<R>, webview_window: WebviewWindow<R>) {
let app_dir = app_handle.path().app_dir();
app_handle
.global_shortcut_manager()
.register("CTRL + U", move || {});
println!("WebviewWindow: {}", webview_window.label());
}

Accessing Managed State
Tauri can manage state using the manage function on tauri::Builder. The state can be accessed on a command using tauri::State:

src-tauri/src/lib.rs
struct MyState(String);

#[tauri::command]
fn my_custom_command(state: tauri::State<MyState>) {
assert_eq!(state.0 == "some state value", true);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
tauri::Builder::default()
.manage(MyState("some state value".into()))
.invoke_handler(tauri::generate_handler![my_custom_command])
.run(tauri::generate_context!())
.expect("error while running tauri application");
}

Accessing Raw Request
Tauri commands can also access the full tauri::ipc::Request object which includes the raw body payload and the request headers.

#[derive(Debug, thiserror::Error)]
enum Error { #[error("unexpected request body")]
RequestBodyMustBeRaw, #[error("missing `{0}` header")]
MissingHeader(&'static str),
}

impl serde::Serialize for Error {
fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
where
S: serde::ser::Serializer,
{
serializer.serialize_str(self.to_string().as_ref())
}
}

#[tauri::command]
fn upload(request: tauri::ipc::Request) -> Result<(), Error> {
let tauri::ipc::InvokeBody::Raw(upload_data) = request.body() else {
return Err(Error::RequestBodyMustBeRaw);
};
let Some(authorization_header) = request.headers().get("Authorization") else {
return Err(Error::MissingHeader("Authorization"));
};

// upload...

Ok(())
}

In the frontend you can call invoke() sending a raw request body by providing an ArrayBuffer or Uint8Array on the payload argument, and include request headers in the third argument:

const data = new Uint8Array([1, 2, 3]);
await **TAURI**.core.invoke('upload', data, {
headers: {
Authorization: 'apikey',
},
});

Creating Multiple Commands
The tauri::generate_handler! macro takes an array of commands. To register multiple commands, you cannot call invoke_handler multiple times. Only the last call will be used. You must pass each command to a single call of tauri::generate_handler!.

src-tauri/src/lib.rs #[tauri::command]
fn cmd_a() -> String {
"Command a"
} #[tauri::command]
fn cmd_b() -> String {
"Command b"
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
tauri::Builder::default()
.invoke_handler(tauri::generate_handler![cmd_a, cmd_b])
.run(tauri::generate_context!())
.expect("error while running tauri application");
}

Complete Example
Any or all of the above features can be combined:

src-tauri/src/lib.rs
struct Database;

#[derive(serde::Serialize)]
struct CustomResponse {
message: String,
other_val: usize,
}

async fn some_other_function() -> Option<String> {
Some("response".into())
}

#[tauri::command]
async fn my*custom_command(
window: tauri::Window,
number: usize,
database: tauri::State<'*, Database>,
) -> Result<CustomResponse, String> {
println!("Called from {}", window.label());
let result: Option<String> = some_other_function().await;
if let Some(message) = result {
Ok(CustomResponse {
message,
other_val: 42 + number,
})
} else {
Err("No result".into())
}
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
tauri::Builder::default()
.manage(Database {})
.invoke_handler(tauri::generate_handler![my_custom_command])
.run(tauri::generate_context!())
.expect("error while running tauri application");
}

import { invoke } from '@tauri-apps/api/core';

// Invocation from JavaScript
invoke('my_custom_command', {
number: 42,
})
.then((res) =>
console.log(`Message: ${res.message}, Other Val: ${res.other_val}`)
)
.catch((e) => console.error(e));

Event System
The event system is a simpler communication mechanism between your frontend and the Rust. Unlike commands, events are not type safe, are always async, cannot return values and only supports JSON payloads.

Global Events
To trigger a global event you can use the event.emit or the WebviewWindow#emit functions:

import { emit } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

// emit(eventName, payload)
emit('file-selected', '/path/to/file');

const appWebview = getCurrentWebviewWindow();
appWebview.emit('route-changed', { url: window.location.href });

Note

Global events are delivered to all listeners

Webview Event
To trigger an event to a listener registered by a specific webview you can use the event.emitTo or the WebviewWindow#emitTo functions:

import { emitTo } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

// emitTo(webviewLabel, eventName, payload)
emitTo('settings', 'settings-update-requested', {
key: 'notification',
value: 'all',
});

const appWebview = getCurrentWebviewWindow();
appWebview.emitTo('editor', 'file-changed', {
path: '/path/to/file',
contents: 'file contents',
});

Note

Webview-specific events are not triggered to regular global event listeners. To listen to any event you must provide the { target: { kind: 'Any' } } option to the event.listen function, which defines the listener to act as a catch-all for emitted events:

import { listen } from '@tauri-apps/api/event';
listen(
'state-changed',
(event) => {
console.log('got state changed event', event);
},
{
target: { kind: 'Any' },
}
);

Listening to Events
The @tauri-apps/api NPM package offers APIs to listen to both global and webview-specific events.

Listening to global events

import { listen } from '@tauri-apps/api/event';

type DownloadStarted = {
url: string;
downloadId: number;
contentLength: number;
};

listen<DownloadStarted>('download-started', (event) => {
console.log(
`downloading ${event.payload.contentLength} bytes from ${event.payload.url}`
);
});

Listening to webview-specific events

import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

const appWebview = getCurrentWebviewWindow();
appWebview.listen<string>('logged-in', (event) => {
localStorage.setItem('session-token', event.payload);
});

The listen function keeps the event listener registered for the entire lifetime of the application. To stop listening on an event you can use the unlisten function which is returned by the listen function:

import { listen } from '@tauri-apps/api/event';

const unlisten = await listen('download-started', (event) => {});
unlisten();

Note

Always use the unlisten function when your execution context goes out of scope such as when a component is unmounted.

When the page is reloaded or you navigate to another URL the listeners are unregistered automatically. This does not apply to a Single Page Application (SPA) router though.

Additionally Tauri provides a utility function for listening to an event exactly once:

import { once } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

once('ready', (event) => {});

const appWebview = getCurrentWebviewWindow();
appWebview.once('ready', () => {});

Note

Events emitted in the frontend also trigger listeners registered by these APIs. For more information, see the Calling Rust from the Frontend documentation.

Listening to Events on Rust
Global and webview-specific events are also delivered to listeners registered in Rust.

Listening to global events

src-tauri/src/lib.rs
use tauri::Listener;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
tauri::Builder::default()
.setup(|app| {
app.listen("download-started", |event| {
if let Ok(payload) = serde_json::from_str::<DownloadStarted>(&event.payload()) {
println!("downloading {}", payload.url);
}
});
Ok(())
})
.run(tauri::generate_context!())
.expect("error while running tauri application");
}

Listening to webview-specific events

src-tauri/src/lib.rs
use tauri::{Listener, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
tauri::Builder::default()
.setup(|app| {
let webview = app.get_webview_window("main").unwrap();
webview.listen("logged-in", |event| {
let session_token = event.data;
// save token..
});
Ok(())
})
.run(tauri::generate_context!())
.expect("error while running tauri application");
}

The listen function keeps the event listener registered for the entire lifetime of the application. To stop listening on an event you can use the unlisten function:

// unlisten outside of the event handler scope:
let event_id = app.listen("download-started", |event| {});
app.unlisten(event_id);

// unlisten when some event criteria is matched
let handle = app.handle().clone();
app.listen("status-changed", |event| {
if event.data == "ready" {
handle.unlisten(event.id);
}
});

Additionally Tauri provides a utility function for listening to an event exactly once:

app.once("ready", |event| {
println!("app is ready");
});

In this case the event listener is immediately unregistered after its first trigger.

To learn how to listen to events and emit events from your Rust code, see the Rust Event System documentation.

## Calling the Frontend from Rust

This document includes guides on how to communicate with your application frontend from your Rust code. To see how to communicate with your Rust code from your frontend, see Calling Rust from the Frontend.

The Rust side of your Tauri application can call the frontend by leveraging the Tauri event system, using channels or directly evaluating JavaScript code.

Event System
Tauri ships a simple event system you can use to have bi-directional communication between Rust and your frontend.

The event system was designed for situations where small amounts of data need to be streamed or you need to implement a multi consumer multi producer pattern (e.g. push notification system).

The event system is not designed for low latency or high throughput situations. See the channels section for the implementation optimized for streaming data.

The major differences between a Tauri command and a Tauri event are that events have no strong type support, event payloads are always JSON strings making them not suitable for bigger messages and there is no support of the capabilities system to fine grain control event data and channels.

The AppHandle and WebviewWindow types implement the event system traits Listener and Emitter.

Events are either global (delivered to all listeners) or webview-specific (only delivered to the webview matching a given label).

Global Events
To trigger a global event you can use the Emitter#emit function:

src-tauri/src/lib.rs
use tauri::{AppHandle, Emitter};

#[tauri::command]
fn download(app: AppHandle, url: String) {
app.emit("download-started", &url).unwrap();
for progress in [1, 15, 50, 80, 100] {
app.emit("download-progress", progress).unwrap();
}
app.emit("download-finished", &url).unwrap();
}

Note

Global events are delivered to all listeners

Webview Event
To trigger an event to a listener registered by a specific webview you can use the Emitter#emit_to function:

src-tauri/src/lib.rs
use tauri::{AppHandle, Emitter};

#[tauri::command]
fn login(app: AppHandle, user: String, password: String) {
let authenticated = user == "tauri-apps" && password == "tauri";
let result = if authenticated { "loggedIn" } else { "invalidCredentials" };
app.emit_to("login", "login-result", result).unwrap();
}

It is also possible to trigger an event to a list of webviews by calling Emitter#emit_filter. In the following example we emit a open-file event to the main and file-viewer webviews:

src-tauri/src/lib.rs
use tauri::{AppHandle, Emitter, EventTarget};

#[tauri::command]
fn open*file(app: AppHandle, path: std::path::PathBuf) {
app.emit_filter("open-file", path, |target| match target {
EventTarget::WebviewWindow { label } => label == "main" || label == "file-viewer",
* => false,
}).unwrap();
}

Note

Webview-specific events are not triggered to regular global event listeners. To listen to any event you must use the listen_any function instead of listen, which defines the listener to act as a catch-all for emitted events.

Event Payload
The event payload can be any serializable type that also implements Clone. Let’s enhance the download event example by using an object to emit more information in each event:

src-tauri/src/lib.rs
use tauri::{AppHandle, Emitter};
use serde::Serialize;

#[derive(Clone, Serialize)] #[serde(rename_all = "camelCase")]
struct DownloadStarted<'a> {
url: &'a str,
download_id: usize,
content_length: usize,
}

#[derive(Clone, Serialize)] #[serde(rename_all = "camelCase")]
struct DownloadProgress {
download_id: usize,
chunk_length: usize,
}

#[derive(Clone, Serialize)] #[serde(rename_all = "camelCase")]
struct DownloadFinished {
download_id: usize,
}

#[tauri::command]
fn download(app: AppHandle, url: String) {
let content_length = 1000;
let download_id = 1;

app.emit("download-started", DownloadStarted {
url: &url,
download_id,
content_length
}).unwrap();

for chunk_length in [15, 150, 35, 500, 300] {
app.emit("download-progress", DownloadProgress {
download_id,
chunk_length,
}).unwrap();
}

app.emit("download-finished", DownloadFinished { download_id }).unwrap();
}

Listening to Events
Tauri provides APIs to listen to events on both the webview and the Rust interfaces.

Listening to Events on the Frontend
The @tauri-apps/api NPM package offers APIs to listen to both global and webview-specific events.

Listening to global events

import { listen } from '@tauri-apps/api/event';

type DownloadStarted = {
url: string;
downloadId: number;
contentLength: number;
};

listen<DownloadStarted>('download-started', (event) => {
console.log(
`downloading ${event.payload.contentLength} bytes from ${event.payload.url}`
);
});

Listening to webview-specific events

import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

const appWebview = getCurrentWebviewWindow();
appWebview.listen<string>('logged-in', (event) => {
localStorage.setItem('session-token', event.payload);
});

The listen function keeps the event listener registered for the entire lifetime of the application. To stop listening on an event you can use the unlisten function which is returned by the listen function:

import { listen } from '@tauri-apps/api/event';

const unlisten = await listen('download-started', (event) => {});
unlisten();

Note

Always use the unlisten function when your execution context goes out of scope such as when a component is unmounted.

When the page is reloaded or you navigate to another URL the listeners are unregistered automatically. This does not apply to a Single Page Application (SPA) router though.

Additionally Tauri provides a utility function for listening to an event exactly once:

import { once } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

once('ready', (event) => {});

const appWebview = getCurrentWebviewWindow();
appWebview.once('ready', () => {});

Note

Events emitted in the frontend also trigger listeners registered by these APIs. For more information, see the Calling Rust from the Frontend documentation.

Listening to Events on Rust
Global and webview-specific events are also delivered to listeners registered in Rust.

Listening to global events

src-tauri/src/lib.rs
use tauri::Listener;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
tauri::Builder::default()
.setup(|app| {
app.listen("download-started", |event| {
if let Ok(payload) = serde_json::from_str::<DownloadStarted>(&event.payload()) {
println!("downloading {}", payload.url);
}
});
Ok(())
})
.run(tauri::generate_context!())
.expect("error while running tauri application");
}

Listening to webview-specific events

src-tauri/src/lib.rs
use tauri::{Listener, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
tauri::Builder::default()
.setup(|app| {
let webview = app.get_webview_window("main").unwrap();
webview.listen("logged-in", |event| {
let session_token = event.data;
// save token..
});
Ok(())
})
.run(tauri::generate_context!())
.expect("error while running tauri application");
}

The listen function keeps the event listener registered for the entire lifetime of the application. To stop listening on an event you can use the unlisten function:

// unlisten outside of the event handler scope:
let event_id = app.listen("download-started", |event| {});
app.unlisten(event_id);

// unlisten when some event criteria is matched
let handle = app.handle().clone();
app.listen("status-changed", |event| {
if event.data == "ready" {
handle.unlisten(event.id);
}
});

Additionally Tauri provides a utility function for listening to an event exactly once:

app.once("ready", |event| {
println!("app is ready");
});

In this case the event listener is immediately unregistered after its first trigger.

Channels
The event system is designed to be a simple two way communication that is globally available in your application. Under the hood it directly evaluates JavaScript code so it might not be suitable to sending a large amount of data.

Channels are designed to be fast and deliver ordered data. They are used internally for streaming operations such as download progress, child process output and WebSocket messages.

Let’s rewrite our download command example to use channels instead of the event system:

src-tauri/src/lib.rs
use tauri::{AppHandle, ipc::Channel};
use serde::Serialize;

#[derive(Clone, Serialize)] #[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "event", content = "data")]
enum DownloadEvent<'a> {
Started {
url: &'a str,
download_id: usize,
content_length: usize,
},
Progress {
download_id: usize,
chunk_length: usize,
},
Finished {
download_id: usize,
},
}

#[tauri::command]
fn download(app: AppHandle, url: String, on_event: Channel<DownloadEvent>) {
let content_length = 1000;
let download_id = 1;

on_event.send(DownloadEvent::Started {
url: &url,
download_id,
content_length,
}).unwrap();

for chunk_length in [15, 150, 35, 500, 300] {
on_event.send(DownloadEvent::Progress {
download_id,
chunk_length,
}).unwrap();
}

on_event.send(DownloadEvent::Finished { download_id }).unwrap();
}

When calling the download command you must create the channel and provide it as an argument:

import { invoke, Channel } from '@tauri-apps/api/core';

type DownloadEvent =
| {
event: 'started';
data: {
url: string;
downloadId: number;
contentLength: number;
};
}
| {
event: 'progress';
data: {
downloadId: number;
chunkLength: number;
};
}
| {
event: 'finished';
data: {
downloadId: number;
};
};

const onEvent = new Channel<DownloadEvent>();
onEvent.onmessage = (message) => {
console.log(`got download event ${message.event}`);
};

await invoke('download', {
url: 'https://raw.githubusercontent.com/tauri-apps/tauri/dev/crates/tauri-schema-generator/schemas/config.schema.json',
onEvent,
});

Evaluating JavaScript
To directly execute any JavaScript code on the webview context you can use the WebviewWindow#eval function:

src-tauri/src/lib.rs
use tauri::Manager;

tauri::Builder::default()
.setup(|app| {
let webview = app.get_webview_window("main").unwrap();
webview.eval("console.log('hello from Rust')")?;
Ok(())
})

If the script to be evaluated is not so simple and must use input from Rust objects we recommend using the serialize-to-javascript crate.

## State Management

In a Tauri application, you often need to keep track of the current state of your application or manage the lifecycle of things associated with it. Tauri provides an easy way to manage the state of your application using the Manager API, and read it when commands are called.

Here is a simple example:

use tauri::{Builder, Manager};

struct AppData {
welcome_message: &'static str,
}

fn main() {
Builder::default()
.setup(|app| {
app.manage(AppData {
welcome_message: "Welcome to Tauri!",
});
Ok(())
})
.run(tauri::generate_context!())
.unwrap();
}

You can later access your state with any type that implements the Manager trait, for example the App instance:

let data = app.state::<AppData>();

For more info, including accessing state in commands, see the Accessing State section.

Mutability
In Rust, you cannot directly mutate values which are shared between multiple threads or when ownership is controlled through a shared pointer such as Arc (or Tauri’s State). Doing so could cause data races (for example, two writes happening simultaneously).

To work around this, you can use a concept known as interior mutability. For example, the standard library’s Mutex can be used to wrap your state. This allows you to lock the value when you need to modify it, and unlock it when you are done.

use std::sync::Mutex;

use tauri::{Builder, Manager};

#[derive(Default)]
struct AppState {
counter: u32,
}

fn main() {
Builder::default()
.setup(|app| {
app.manage(Mutex::new(AppState::default()));
Ok(())
})
.run(tauri::generate_context!())
.unwrap();
}

The state can now be modified by locking the mutex:

let state = app.state::<Mutex<AppState>>();

// Lock the mutex to get mutable access:
let mut state = state.lock().unwrap();

// Modify the state:
state.counter += 1;

At the end of the scope, or when the MutexGuard is otherwise dropped, the mutex is unlocked automatically so that other parts of your application can access and mutate the data within.

When to use an async mutex
To quote the Tokio documentation, it’s often fine to use the standard library’s Mutex instead of an async mutex such as the one Tokio provides:

Contrary to popular belief, it is ok and often preferred to use the ordinary Mutex from the standard library in asynchronous code … The primary use case for the async mutex is to provide shared mutable access to IO resources such as a database connection.

It’s a good idea to read the linked documentation fully to understand the trade-offs between the two. One reason you would need an async mutex is if you need to hold the MutexGuard across await points.

Do you need Arc?
It’s common to see Arc used in Rust to share ownership of a value across multiple threads (usually paired with a Mutex in the form of Arc<Mutex<T>>). However, you don’t need to use Arc for things stored in State because Tauri will do this for you.

In case State’s lifetime requirements prevent you from moving your state into a new thread you can instead move an AppHandle into the thread and then retrieve your state as shown below in the “Access state with the Manager trait” section. AppHandles are deliberately cheap to clone for use-cases like this.

Accessing State
Access state in commands #[tauri::command]
fn increase*counter(state: State<'*, Mutex<AppState>>) -> u32 {
let mut state = state.lock().unwrap();
state.counter += 1;
state.counter
}

For more information on commands, see Calling Rust from the Frontend.

Async commands
If you are using async commands and want to use Tokio’s async Mutex, you can set it up the same way and access the state like this:

#[tauri::command]
async fn increase*counter(state: State<'*, Mutex<AppState>>) -> Result<u32, ()> {
let mut state = state.lock().await;
state.counter += 1;
Ok(state.counter)
}

Note that the return type must be Result if you use asynchronous commands.

Access state with the Manager trait
Sometimes you may need to access the state outside of commands, such as in a different thread or in an event handler like on_window_event. In such cases, you can use the state() method of types that implement the Manager trait (such as the AppHandle) to get the state:

use std::sync::Mutex;
use tauri::{Builder, Window, WindowEvent, Manager};

#[derive(Default)]
struct AppState {
counter: u32,
}

// In an event handler:
fn on_window_event(window: &Window, \_event: &WindowEvent) {
// Get a handle to the app so we can get the global state.
let app_handle = window.app_handle();
let state = app_handle.state::<Mutex<AppState>>();

    // Lock the mutex to mutably access the state.
    let mut state = state.lock().unwrap();
    state.counter += 1;

}

fn main() {
Builder::default()
.setup(|app| {
app.manage(Mutex::new(AppState::default()));
Ok(())
})
.on_window_event(on_window_event)
.run(tauri::generate_context!())
.unwrap();
}

This method is useful when you cannot rely on command injection. For example, if you need to move the state into a thread where using an AppHandle is easier, or if you are not in a command context.

Mismatching Types
Caution

If you use the wrong type for the State parameter, you will get a runtime panic instead of compile time error.

For example, if you use State<'_, AppState> instead of State<'_, Mutex<AppState>>, there won’t be any state managed with that type.

If you prefer, you can wrap your state with a type alias to prevent this mistake:

use std::sync::Mutex;

#[derive(Default)]
struct AppStateInner {
counter: u32,
}

type AppState = Mutex<AppStateInner>;

However, make sure to use the type alias as it is, and not wrap it in a Mutex a second time, otherwise you will run into the same issue.

## HTTP Client

GitHub
npm
crates.io
API Reference
Make HTTP requests with the http plugin.

Supported Platforms
This plugin requires a Rust version of at least 1.77.2

Platform Level Notes
windows
linux
macos
android
ios
Setup
Install the http plugin to get started.

Automatic
Manual
Use your project’s package manager to add the dependency:

npm
yarn
pnpm
deno
bun
cargo
cargo tauri add http

Usage
The HTTP plugin is available in both Rust as a reqwest re-export and JavaScript.

JavaScript
Configure the allowed URLs

src-tauri/capabilities/default.json
{
"permissions": [
{
"identifier": "http:default",
"allow": [{ "url": "https://*.tauri.app" }],
"deny": [{ "url": "https://private.tauri.app" }]
}
]
}

For more information, please see the documentation for Permissions Overview

Send a request

The fetch method tries to be as close and compliant to the fetch Web API as possible.

import { fetch } from '@tauri-apps/plugin-http';

// Send a GET request
const response = await fetch('http://test.tauri.app/data.json', {
method: 'GET',
});
console.log(response.status); // e.g. 200
console.log(response.statusText); // e.g. "OK"

Note

Forbidden request headers are ignored by default. To use them you must enable the unsafe-headers feature flag:

src-tauri/Cargo.toml
[dependencies]
tauri-plugin-http = { version = "2", features = ["unsafe-headers"] }

Rust
In Rust you can utilize the reqwest crate re-exported by the plugin. For more details refer to reqwest docs.

use tauri_plugin_http::reqwest;

let res = reqwest::get("http://my.api.host/data.json").await;
println!("{:?}", res.status()); // e.g. 200
println!("{:?}", res.text().await); // e.g Ok("{ Content }")

Default Permission
This permission set configures what kind of fetch operations are available from the http plugin.

This enables all fetch operations but does not allow explicitly any origins to be fetched. This needs to be manually configured before usage.

Granted Permissions
All fetch operations are enabled.

This default permission set includes the following:
allow-fetch
allow-fetch-cancel
allow-fetch-send
allow-fetch-read-body
allow-fetch-cancel-body
Permission Table
Identifier Description
http:allow-fetch

Enables the fetch command without any pre-configured scope.

http:deny-fetch

Denies the fetch command without any pre-configured scope.

http:allow-fetch-cancel

Enables the fetch_cancel command without any pre-configured scope.

http:deny-fetch-cancel

Denies the fetch_cancel command without any pre-configured scope.

http:allow-fetch-cancel-body

Enables the fetch_cancel_body command without any pre-configured scope.

http:deny-fetch-cancel-body

Denies the fetch_cancel_body command without any pre-configured scope.

http:allow-fetch-read-body

Enables the fetch_read_body command without any pre-configured scope.

http:deny-fetch-read-body

Denies the fetch_read_body command without any pre-configured scope.

http:allow-fetch-send

Enables the fetch_send command without any pre-configured scope.

http:deny-fetch-send

Denies the fetch_send command without any pre-configured scope.

Edit page
