# Currently/TODO
LATEST: Having weird problem, will upgrade to 0.12.0
- Remove Bullet when outside of screen
- no reliable way to see if standing on ground

- Shooting: Bullet component
    - Carries an "explosive" - a routine to explode environment?
    - movement behaviour (actually just need an initial movement speed
        - maybe requires to generalize MoveSys (to not just be for players)
- Taking it further:
    - imagine a laser - not an object
    - give it (and other weapons) a certain degree of destruction
        - partial destruction of tiles? -> common air? Rather common ground.

- How to deal with player actions? Maybe through ECS? For example letting Player have an array of actions

- Integrate Color + Shape? YES

- Robust and adjustable granularity of sweeping in `tilenet`.
     - say a laser weapon... should absolutely sweep all tiles.
       maybe we need a method to sweep a certain diameter

# TileNet
- solve: just return new Collable rather than mutate?
  - could alleviate e.g. having the result in RayCollable
- should maybe operate with only u32 altogether. Or i32 and always check bounds
- make solve() return the result rather than mutate? (??)


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


