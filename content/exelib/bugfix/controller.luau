--!strict
-- BugFix's dimming controller, as EXE5 and EXE6 share it (EXE6's effect object #0x3B, `sub_80E4910`; EXE5's #0x3B,
-- 0x080E4940, the same code but for its effect; EXE6's docs/engine/dimming-chips.md §5.1): the usual dimming phases
-- (@exelib/dimming), with an effect (EXE6's `sub_80E4954`, EXE5's 0x080E4984) that puts the glow over the user's
-- side's navi (@exelib/bugfix/glow), fixes the user's side's NaviCust bugs, and waits for the glow. EXE6's also spends
-- the Beast Out counter again unless the navi has one (`sub_8014446` / `sub_801443C`); EXE5 has no Beast Out. (EXE5's
-- fix, 0x080E49DA, leaves NaviStats +0x63 alone, the hand's shrink turn: a bug EXE5 hasn't, which stays 0.)
--
-- Each game makes its own kind (`controller.make`) with its glow and whether it has Beast Out.

local dimming_chips = require("@exelib/dimming")

local controller = {}

export type State = {
    -- The glow runs (the original's Param2, which the glow clears).
    busy: boolean,
}

export type Look = {
    -- The kind's id.
    id: string,
    -- The game's glow's spawn (@exelib/bugfix/glow).
    glow: (spawner: Object, variant: number, holder: Object) -> Object?,
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
-- The glow's variant (`off_80C4930`'s first).
local BUG_FIX = 0

function controller.make(look: Look): Controller
    -- `sub_80E4954`.
    local function effect(me: Object)
        if me.phase_init == 0 then
            -- sub_80C4AEC
            look.glow(me, BUG_FIX, me)
            -- sub_80E49C4
            local user = me.related1
            if not user then
                error("BugFix's controller has no user (sub_80E49C4 reads through a null pointer)")
            end
            battle.clear_navicust_bugs(user.alliance)
            -- sub_801E658: the emotion portrait's bug face goes, on every
            -- console (its flicker draws the console's RNG1).
            battle.clear_emotion_window_glitch()
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
