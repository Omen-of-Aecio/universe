# TileNet
- solve: just return new Collable rather than mutate?
  - could alleviate e.g. having the result in RayCollable
- should maybe operate with only u32 altogether.


# Rendering
 - Render tilenet 
 - Render polygons (system)
 - Render lines

# Resources
  - RenderConfig
  - GameConfig
  - TileNet
  - Display

  - `tilenet_ren::Ren`
  - `polygons::ren::Ren`


# Structure
Client
  Networking
  Systems
  CliGame
    Components

Server
  SrvGame
    Components
  Systems


