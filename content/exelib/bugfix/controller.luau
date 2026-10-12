--!strict
-- BugFix's dimming controller, as EXE4, EXE5 and EXE6 share it (EXE6's effect object #0x3B, `sub_80E4910`; EXE5's
-- #0x3B, 0x080E4940, and EXE4's, 0x080E6388, the same code but for their effects; EXE6's docs/engine/dimming-chips.md
-- §5.1): the usual dimming phases (@exelib/dimming), with an effect (EXE6's `sub_80E4954`, EXE5's 0x080E4984, EXE4's
-- 0x080E63CC) that puts the glow over the user's side's navi, fixes the user's side's NaviCust bugs, and waits for the
-- glow. EXE6's also spends the Beast Out counter again unless the navi has one (`sub_8014446` / `sub_801443C`); EXE5
-- and EXE4 have no Beast Out.
--
-- Each game makes its own kind (`controller.make`) with its glow's spawn, its fix and whether it has Beast Out. EXE6's
-- and EXE5's fix is `controller.clear_bugs` (EXE5's, 0x080E49DA, leaves NaviStats +0x63 alone, the hand's shrink turn:
-- a bug EXE5 hasn't, which stays 0); EXE4's is its own (its stats are its own layout), with no emotion window to
-- clear. The glow's flashes (`controller.flashes`) are the three games' alike.

local dimming_chips = require("@exelib/dimming")

local controller = {}

export type State = {
    -- The glow runs (the original's Param2, which the glow clears).
    busy: boolean,
}

export type Look = {
    -- The kind's id.
    id: string,
    -- The game's glow over `spawner`'s side's navi with BugFix's flashes (EXE6's `sub_80C4AEC`); `holder` is busy until
    -- it is done.
    glow: (spawner: Object, holder: Object) -> Object?,
    -- What fixes side `user`'s side's bugs (EXE6's `sub_80E49C4` and `sub_801E658`: `controller.clear_bugs`).
    fix: (user: Object) -> (),
    -- EXE6's: the Beast Out counter is spent again.
    beast_out: boolean,
}

export type Controller = {
    kind: Kind,
    -- `sub_80E49A2`: the chip's `dimming` hook.
    spawn: (user: Object, spec: DimmingChipSpec) -> Object?,
}

-- What the effect's first tick sets the timer to (nothing counts it).
local TIMER = 0x3C

-- The glow's flashes' ticks, and the first flash's.
local FLASH_TICKS, FIRST_FLASH = 0x78, 2

-- The glow's flashes, its first action (EXE6's `sub_80C4958`, EXE5's 0x080C4420, EXE4's 0x080CD058): on the first tick
-- the navi vanishes (`hide`); then a flash (white, `flash`'s sound) whenever Timer2 runs out, Timer2 starting again at
-- a quarter of the ticks so far; at 120 ticks, action 4.
function controller.flashes(flash: Sound, hide: (me: Object) -> ()): (me: Object) -> ()
    return function(me: Object)
        me.sprite.white = false
        if me.phase_init == 0 then
            hide(me)
            me.timer = 0
            me.timer2 = FIRST_FLASH
            me.phase_init = 4
        end
        local t2 = me.timer2 - 1
        me.timer2 = t2
        if t2 < 0 then
            me.timer2 = bit32.rshift(me.timer, 2)
            me.sprite.white = true
            battle.play_sound(flash)
        end
        local t = me.timer + 1
        me.timer = t
        if t >= FLASH_TICKS then
            me:set_action(4)
        end
    end
end

-- EXE6's and EXE5's fix: `sub_80E49C4`, the side's bugs (`battle.clear_bugs`); `sub_801E658`, the emotion portrait's
-- bug face goes, on every console (its flicker draws the console's RNG1).
function controller.clear_bugs(user: Object)
    battle.clear_bugs(user.alliance)
    battle.clear_emotion_window_glitch()
end

function controller.make(look: Look): Controller
    -- `sub_80E4954`.
    local function effect(me: Object)
        if me.phase_init == 0 then
            -- sub_80C4AEC
            look.glow(me, me)
            local user = me.related1
            if not user then
                error("BugFix's controller has no user (sub_80E49C4 reads through a null pointer)")
            end
            look.fix(user)
            if look.beast_out then
                -- sub_8014446 / sub_801443C: the Beast Out counter is spent
                -- again unless the navi has one.
                user.tired = battle.navi(user.alliance).beast_out_counter == 0
            end
            me.timer = TIMER
            me.phase_init = 4
        end
        if not (me.state :: State).busy then
            dimming_chips.done(me)
        end
    end

    local kind = new.kind {
        id = look.id,
        pool = "effect",
        state = { busy = "bool" },
        update = dimming_chips.phases { name = "BugFix", effect = effect },
    }

    local function spawn(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(kind, user, spec)
    end

    return { kind = kind, spawn = spawn }
end

return controller
