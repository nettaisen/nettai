--!strict
-- The NaviCust supports' dimming controller, as EXE5 and EXE6 share it (EXE6's effect object #0x79, `sub_80E8FE0`; EXE5's
-- #0x74, 0x080E8F50, the same code), of a game's `SupportsLook` (`controller.make`). In link battles a NaviCust
-- program's support acts once: Rush eats a chip the opponent uses and Beat snatches a Mega or Giga chip (their host is
-- the chip user's opponent), Tango heals her host when its HP drops to a quarter. The rules turn the chip back
-- (`sub_8010740`, `sub_80106C0`) or holds the navi (`sub_8010660`), spawns this controller on the host's panel
-- (`sub_80E90FE`; the role `kinds.support`) and starts a dimming no one can cut in on (`sub_800BF16`).
--
-- The controller dims the screen, shows the telop (the support's chip record, which the game keeps at +0x30), then:
-- Rush's and Beat's host warps out for 31 ticks (Tango's doesn't), the support comes and the controller waits for it
-- to go, waits 31 ticks, the host warps back in (not Tango's), 31 ticks, and the screen undims. See
-- docs/engine/chips.md §2.10.
--
-- The rules set its state as it spawns it: which support (the original's first parameter), the chip Rush eats (its
-- third and fourth) and the telop's chip. Related object 1 is the host.

local dimming_chips = require("../dimming")

export type Support = "rush" | "beat" | "tango"

export type State = {
    -- Which support comes.
    support: Support,
    -- The support is out: it sets this as it comes and clears it as it
    -- goes (the original's second parameter).
    out: boolean,
    -- The chip Rush eats, if any (the engine's for every game; a Rush
    -- without a spared chip, EXE5's, keeps none).
    eaten: Chip?,
    -- The chip the telop names (+0x30: Rush's, Beat's or Tango's, the
    -- roles' `chips.rush`, `beat` and `tango`), which the rules set.
    -- (The port's telop banner doesn't show a name yet.)
    telop_chip: Chip?,
}

local controller = {}

-- How long each wait lasts: the timer counts down past zero, so 31 ticks.
local WAIT = 0x1E

-- A wait's tick (`ldrh`, `sub`, `strh`, `bge`): false once it counts past
-- zero.
local function waiting(me: Object): boolean
    local t = me.timer - 1
    me.timer = int.u16(t)
    return t >= 0
end

local function next_phase(me: Object, phase: number)
    me.phase, me.phase_init = phase, 0
end

-- A game's supports' controller, of `look`.
function controller.make(look: SupportsLook): Kind
    -- The support's spawner (`off_80E90A4`, EXE5's 0x080E9010, by the first
    -- parameter).
    local function send(me: Object, s: State)
        local support = s.support
        if support == "rush" then
            look.rush.spawn(me, s.eaten)
        elseif support == "beat" then
            look.beat.spawn(me)
        else
            look.tango.spawn(me)
        end
    end

    -- `sub_80E9024` (action 8; EXE5's 0x080E8F94), by phase.
    local function effect(me: Object)
        local s = me.state :: State
        local p = me.phase
        if p == 0 then
            -- sub_80E9048: the host warps out (not Tango's).
            if me.phase_init == 0 then
                if s.support == "tango" then
                    next_phase(me, 4)
                    return
                end
                navi_chip.warp(me.related1 :: Object, true)
                me.timer = WAIT
                me.phase_init = 4
            end
            if not waiting(me) then
                next_phase(me, 4)
            end
        elseif p == 4 then
            -- sub_80E9074: the support comes; the controller waits for it to
            -- go (a support that couldn't come never was out).
            if me.phase_init == 0 then
                send(me, s)
                me.phase_init = 4
            end
            if not s.out then
                next_phase(me, 8)
            end
        elseif p == 8 then
            -- sub_80E90B0
            if me.phase_init == 0 then
                me.timer = WAIT
                me.phase_init = 4
            end
            if not waiting(me) then
                next_phase(me, 0xC)
            end
        elseif p == 0xC then
            -- sub_80E90CE: the host warps back in; then the undim.
            if me.phase_init == 0 then
                if s.support ~= "tango" then
                    navi_chip.warp(me.related1 :: Object, false)
                end
                me.timer = WAIT
                me.phase_init = 4
            end
            if not waiting(me) then
                dimming_chips.done(me)
            end
        else
            error(string.format("the supports' phase %#x reads past its table (off_80E9038)", p))
        end
    end

    return define.kind {
        id = look.id,
        pool = "effect",
        state = { support = { "rush", "beat", "tango" }, out = "bool", eaten = "chip", telop_chip = "chip" },
        -- `sub_80E8FE0`.
        update = dimming_chips.phases { name = "the supports", effect = effect },
    }
end

return controller
