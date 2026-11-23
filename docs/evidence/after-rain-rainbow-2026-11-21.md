# After Rain's rainbow — November 21, 2026

PC port addition (Weather Settings' After Rain, `gpu_renderer/rain.wgsl`).

- The bow is a ring in the sky 0.705 to 0.760 radians from a centre set a little under the
  horizon, in the direction the camera faced when the weather or the map was entered. It stays
  there in the world as the camera turns, and fades out toward the horizon and overhead.
- It was drawn as three overlapping bands, one each of pure red, green and blue, added to the
  sky: red outside, a green middle, blue inside, and no other colour.
- It now runs through a primary rainbow's seven colours from its inner edge to its outer: violet,
  indigo, blue, green, yellow, orange, red (`rainbow_colour`), each blending into the next, with
  soft inner and outer edges.
- It tints the sky more than it lights it: each colour is added less 0.55 of its own brightness,
  so yellow and green stay colours over a pale sky instead of washing it out. The sky inside the
  bow is a little brighter than the sky outside it, as inside a primary bow.
- Seen in a scripted run at an open spot with the weather picked after arriving
  (`SKATE2_DIAGNOSTIC_WEATHER=AfterRain@1500`), beside the same run of the build before: the old
  bow is pale pink, green and blue; the new one reads red, yellow, green, blue and violet from
  the outside in.
