--!strict
-- EXE4's barrier visual (effect object 7, 0x080E2418: EXE5's and EXE6's code, @exelib/barriers/visual, with EXE4's
-- differences): it sits on its navi and follows the navi's barrier (the collision data's): up, it shows; worn down,
-- it goes down (and comes back up when the barrier has HP again); with the barrier gone, it goes. The navi links it
-- (`barrier_visual`: EXE4's object +0x50, which 0x08012DEC clears with the barrier), and hides and shows it with
-- itself (its `shown`, +0x0C). As EXE5's: no blown action (wind takes EXE4's barrier away at once). EXE4's own: no
-- test of its navi's being on the field (0x080E24A4), its sprite stepped paused or not (0x080144BC), running while
-- paused only until the fight starts, placed by a table of offsets by its navi's number (NaviStats +0x23: 0x080E2572,
-- or 0x080E2558 for an aura, type 6 on), not an attach point, and its spawn (0x080E2622) taking its navi's side
-- alone. Its looks' parts another navi than MegaMan hides (0x080E247A: 0x02000000) no netbattle reaches.

local common = require("@exelib/barriers/visual")

local visual = {}

local made = common.make {
    id = "barrier-visual",
    blows = false,
    hides_off_field = false,
    -- 0x080E2572 and 0x080E2558 by the navi's number: MegaMan's rows are none, and EXE4's netbattles have no other
    -- navi.
    offset = function(_owner: Object, _b: Barrier): (number, number)
        return 0, 0
    end,
    stops_running_paused_in_fight = true,
    steps_paused = true,
    side_only = true,
}

visual.kind = made.kind

-- 0x080E2622: `b`'s visual on `navi`, which links it; it runs while paused (until the fight starts) and shows with
-- its navi. (Spawned at the caller's registers; its init places it.)
visual.spawn = made.spawn

return visual
