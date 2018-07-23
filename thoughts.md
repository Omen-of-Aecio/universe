# Currently/TODO
- UniqueID on client side.. they are allocated by the server so what do I do about that... have to wait with adding them until next snapshot? Or add some temporary version for prediction. Which can go when we `maintain` the world.

- SNAPSHOT
    maybe would make more sense to send a snapshot with Options...




- while we can serialize the Snapshot...
    do we really have to deserialize it and then loop & apply to the ECS system?
    Is it possible to do so while deserializing? LATER. OPTIMIZATION.

- Remove Bullet when outside of screen
- **no reliable way to see if standing on ground** (need for ground friction)

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

- Would be nice if the motion of Player and Bullet, etc, were just mere physics.
        But the player system would 

# Network
- add timestamps, and try to keep in sync?
`https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking`
- general idea: on a listen server, could make it possible to visualize server vs. client hitboxes ETC
###Entity Interpolation
- server has a tick rate, but clients receive packets at some other rate depending on BW
- delta compression in snapshots
- render back in time - **interpolate** between already-received snapshots
    - this lag is compensated for in the server
### Input Prediction (Client)
    "the client runs exactly the same code and rules the server will use to process the user commands"
    - gradually correct errors over a period of time to hide prediction errors
### Lag Compensation (Server)
When user command/input is received:
 1. Estimate `Command Execution Time = Current Server Time - Packet Latency - Client View Interpolation`
 2. Move all players back to that time (keeps track of history for last second)
 3. Move all players back

## Roadmap
1. Entity interpolation, snapshots, delta time
2. Input Preduction
3. Delta compression
(4. Lag Compensation)

# Snapshots
- specify which components of which entities are needed
- for example:
     (player, col, pos, shape)
     (bullet, col, pos, shape)
- compare before sending - send only if changed (CMP w/ what is sent previously PER CLIENT)
     for example, `col`, `shape` will likely not change.
     However, make sure that the Player(id) or Bullet(id) is sent!
 Sorta intra,inter prediction pattern VS Individually what each client needs...
        --> first one!! Client should request full snapshot!
```
enum Type {Player(id), Bullet(id)}
struct Entity {
    type: Type,
    components: Vec<Component>
}
enum Component {
    Pos(Pos),
    Vel(Vel),
    Force(Force),
    ....
}
```

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


