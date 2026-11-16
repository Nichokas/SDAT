# Glass benchmark (Android)

A standalone app that answers one question: can SDAT run at 60 fps on a given phone with Glasmio's glass, and if not, what limits it? It needs no TIDAL account and no network. It draws its own album-like covers and recreates SDAT's heaviest screens with the same Glasmio, Compose and Coil versions, which it reads from `sdat-android/gradle/libs.versions.toml`.

## Scenes

Each scene animates itself while the app records every frame with `FrameMetrics`.

| Scene | What it stresses |
| --- | --- |
| Discover, steady pan | The cover plane moving at 900 dp/s under the header buttons, tab bar and mini player. |
| Discover, fast fling | 3,500 dp/s with Coil's memory cache off, so every cover that enters is decoded again. |
| Collection list scroll | A 400-row cover list at 2,200 dp/s under the large-title header, with the tab bar morphing every 2 s. |
| Album page scroll | `DetailPage`'s blurred, zoomed cover backdrop, with glass Play and Shuffle buttons scrolling over it. |
| Player | The player's 80 dp blurred backdrop, with the pager swiping between Now Playing and the queue and the slider moving. |
| Worst case | The fast fling with live glass and two busy cores, standing in for playback, a download and the core's network work. |

The Discover and Collection scenes run three times. The first run uses live glass, as SDAT ships. The second uses `GlassAppearance(reduceTransparency = true)`, which draws the same components as solid fills and samples nothing. The third drops Glasmio's chrome entirely. The album and player scenes have no glass-free form in SDAT, so they run only live and reduced.

The player runs inside the activity window, without `GlasmioBottomSheet`'s own window, because `FrameMetrics` sees only one window.

## Run it

1. Build the APK. Agents use `scripts/depot-run.sh benchmark`. Anyone can use `./gradlew assembleRelease` in this folder, or download the `sdat-glass-benchmark` artifact of the `Benchmark APK` workflow.
2. Install `app-release.apk`. It is an R8 release build signed with a debug key, so it installs beside SDAT.
3. Charge the phone, close other apps, open SDAT Bench, and choose the full run (3 rounds, about 6 minutes). Do not touch the screen.
4. Share or copy the report. It is also saved to `Android/data/com.nichokas.sdat.bench/files/results/`.

## Read the report

The results screen opens with the score: the worst case, the median fps of the slowest scene; the 1% low, the rate the slowest 1% of frames in any scene could hold; and how many scenes hold 60 fps. A chart below shows every scene: the bar is its median fps, the white tick its 1% low and the dashed line 60 fps. The same score heads the shared text.

- `60 fps OK` means a median of at least 57 frames a second and at most 5% of frames slower than 16.7 ms.
- `frames >16.7 ms` counts frames that would miss a 60 Hz deadline, whatever the display's rate. `missed display deadline` uses the phone's own rate, so it is higher on a 90 or 120 Hz screen.
- `p90 by stage` splits each frame into the main thread (input, animation, layout, draw), the RenderThread (sync, command issue, swap) and the GPU. The slowest one is the `limiter`.
- `first-use worst frame` is the worst frame while each scene first appeared. A large value there is a shader compile or a first decode, which users feel when they open a screen.
- Each round runs the scenes in a different order, so heat and caches favour none of them. Medians come with the min-max across rounds. A difference smaller than that spread is no difference.
- If the report says a run ended at moderate thermal status or worse, the phone throttled and later numbers read slower than the phone is.

What the result decides:

- Live glass misses 60 fps but reduced transparency meets it: glass is the cost. A low-end mode with reduced transparency keeps Glasmio.
- Reduced transparency and no glass also miss it: the limit is not the glass. Replacing Glasmio would not help, and the stage breakdown names what does.
- Live glass meets it, worst case included: Glasmio is not the problem on that phone.
