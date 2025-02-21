File Structure
====================
Gantry uses a single root for storing all the configuration files, metadata and gcode files. By default, this is located at ~/.gantry

- __~/.gantry__
  - Gantry.toml
  - __instance0__
    - printer.cfg
    - __gcodes__
      - benchy.gcode
      - __build__
        - benchy.bgcode
        - benchy.meta
      - __thumbnails__
        - benchy.jpg
      
    - __extensions__
      - __MyExtension__
        - Gantry.toml
        - main.wasm
  - __instance1__
    - ...