--!strict
-- The navi chip controllers (EXE6's and EXE5's navi chips, action 0x1B and 0x41: their one effect object #0x10,
-- `sub_80E17E8`; EXE4's navi chips, the dimming chips' action 0x0C, its navi variants: each navi's effect object,
-- 0x080E2B54 and its copies; EXE5's Phoenix and DethPhnx, its effect objects #0x0D and #0x15): a navi chip's
-- controller, run as a dimming chip's, with its own effect. Its actions: the dim; AntiNavi's check (`sub_800BDB2`);
-- the chip's name (`sub_800BA8A`); the effect; the undim. The effect is the game's course:
--
-- - by phase (0, 4, 8, 0xC, ...), steps: a wait (counted from the step's first tick until it falls below 0: one tick
--   more than it says), the user warping out or back in as it starts; or the navi, which the step waits on. Then the
--   undim. EXE5's Phoenix: a wait, the phoenix, two waits (the user stays). EXE4's longer navis: the user warps out
--   (30), the navi, then a wait and the user warping back in (30), or only the warp, or neither (Roll brings her user
--   back).
-- - EXE4's short course (TopMan's 0x080E2B9C and its copies), counted in the phase's init byte: the user warps out;
--   a wait; the navi, which it waits on, then a second wait from the navi's leaving; then the undim, the user warping
--   back in as it starts.
--
-- AntiNavi (the other side's defensive chip, a trap) turns a chip of the navi chips' block around: a sparkle on the
-- controller's panel, 31 ticks, AntiNavi's telop, and the chip changes sides, its navi coming for AntiNavi's user
-- (docs/engine/dimming-chips.md §2). The block is the original's chips 0xDD to 0x118, which the content tells by the
-- chip's `navi` flag, or the spec's `navi_block` where it differs. A game may keep the chips of the block as their
-- navis come (EXE5's last navi chip used, `keep`).
--
-- Each chip makes its own controller, with its own navi (`spawn`: what differs between a navi's chips, its palette and
-- the like, is the chip's), and its `dimming` hook is that controller's `hook`; the navi tells the controller it is
-- done through `navi_chip.navi_left` (the kind's `navi_left`) or `controller.done`.

local dimming_chips = require("@exelib/dimming")

local controller = {}

-- The controller's state (`NaviChipControllerState`): the chip, which its telop names and AntiNavi checks (+0x30),
-- and its Atk+ bonus (+0x32), which the navi's damage adds; whether the navi is out (EXE5's +0x18, EXE4's +0x05 or
-- ExtraVars+0: the flag its spawn sets and its end clears). (EXE4's controllers keep the chip's first parameter, its
-- navi's level, which their navis read for their palettes and the like: here each chip states those itself.)
type State = NaviChipControllerState
type Step = NaviChipStep
type Short = NaviChipShortCourse

export type Spec = {
    -- The kind's id, and its name for the errors past its tables.
    id: string,
    name: string,
    -- The chip's navi for `user`, on the controller's panel, of the user's side, with the damage word (bonus
    -- included), reporting to the controller: nil when it couldn't come.
    spawn: (controller: Object, user: Object, damage: number) -> Object?,
    -- The effect's course: steps by phase, or EXE4's short one.
    steps: { Step }?,
    short: Short?,
    -- How the controller's user warps out (`out`) or back in (EXE4's 0x080E2D26 by its side; EXE6's and EXE5's actor
    -- #0x2D for the user, `sub_80E1332`): where a course warps.
    warp: ((controller: Object, out: boolean) -> ())?,
    -- Whether the chip is in the navi chips' block (AntiNavi turns it back; a game keeps it): nil, by its `navi` flag.
    navi_block: boolean?,
    -- What the game keeps of a chip of the block as its navi comes (`sub_80E1880`: EXE5's last navi chip used): the
    -- chip, the element and the damage word, bonus included, the navi came with.
    keep: ((chip: Chip?, element: number, damage: number) -> ())?,
}

-- What `make` hands back (`NaviChipController`): the kind and the chip's `dimming` hook.
type Controller = NaviChipController

-- The actions: the dim, AntiNavi, the name, the effect, the undim.
local DIM, ANTI_NAVI, NAME, EFFECT, UNDIM = 0x0, 0x4, 0x8, 0xC, 0x10

-- The navi is done (it writes 0 through the pointer its spawn was given).
function controller.done(me: Object)
    (me.state :: State).busy = false
end

-- How long the sparkle shows before AntiNavi's telop, in ticks after the first (`sub_800BE0C`).
local ANTI_NAVI_WAIT = 0x1E

-- The other side's defensive chip is AntiNavi.
local function anti_navi_waits(side: number): boolean
    local trap = battle.linked(bit32.bxor(side, 1)).chip
    return trap ~= nil and trap.trap == "anti_navi"
end

-- Count the timer down: true once it fell below 0.
local function ran_out(me: Object): boolean
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left < 0
end

-- The chip is in the navi chips' block (AntiNavi turns it back; a game keeps it): `navi_block`, else its `navi` flag.
function controller.in_block(chip: Chip?, navi_block: boolean?): boolean
    if navi_block ~= nil then
        return navi_block
    end
    return chip ~= nil and table.find(chip.flags or {}, "navi") ~= nil
end

-- `sub_800BDB2` (`off_800BDC4` by phase), a navi chip controller's action after the dim: on to the next action (the
-- name), unless the other side's AntiNavi turns a chip of the block (`in_block`) around: the side's dimming runs its
-- effect from now; the trap's mark over the controller's panel; 31 ticks after the first (`sub_800BE0C`); AntiNavi's
-- telop as its user's side sees it, without damage; then the chip changes sides (the side's dimming is over, the
-- controller takes the other side, which owns, runs and started the dimming, with AntiNavi's user as the chip's user,
-- on its panel), AntiNavi is spent, and the name follows, the other side's (`sub_800BE2C`).
function controller.anti_navi(me: Object, in_block: boolean)
    local side = me.alliance
    local p = me.phase
    if p == 0 then
        -- sub_800BDD0
        if not (in_block and anti_navi_waits(side)) then
            me:set_action(me.action + 4)
            return
        end
        dimming.run(side)
        battle.trap_mark(me)
        me.phase, me.phase_init = 4, 0
    elseif p == 4 then
        if me.phase_init == 0 then
            me.timer = ANTI_NAVI_WAIT
            me.phase_init = 4
            return
        end
        if ran_out(me) then
            me.phase, me.phase_init = 8, 0
        end
    elseif p == 8 then
        if me.phase_init == 0 then
            -- (AntiNavi: the chip its user's record holds.)
            local other = bit32.bxor(side, 1)
            dimming.chip_telop(other, battle.linked(other).chip)
            me.phase_init = 4
            return
        end
        if not dimming.telop_done() then
            return
        end
        dimming.turn(me)
        local taker = me.alliance
        -- sub_802CE78: AntiNavi's record's user.
        local owner = battle.linked(taker).owner
        if owner then
            me.panel_x, me.panel_y = owner.panel_x, owner.panel_y
            me.related1 = owner
        end
        -- sub_802CEA6
        battle.clear_linked(taker)
        me:set_action(me.action + 4)
    else
        error(string.format("a navi chip controller's phase %#x reads past sub_800BDB2's table (off_800BDC4)", p))
    end
end

-- `sub_800BA8A`, the action after AntiNavi's: the chip's name; once the side's dimming runs, the next action (the
-- effect), unless the user was deleted (then the one after, the undim: after AntiNavi turned the chip around the
-- record has no user, and the game reads its HP through the null pointer, from the BIOS, whose open-bus value is never
-- 0), or the other side's AntiNavi turns a chip of the block around again (back to `sub_800BDB2`).
function controller.name(me: Object, chip: Chip?, in_block: boolean)
    if not dimming.telop_running(me, chip) then
        return
    end
    local user = dimming.user(me.alliance)
    if user and user.hp == 0 then
        -- (dword_200F3B8[side] = 1: never read.)
        me:set_action(me.action + 8)
        return
    end
    if in_block and anti_navi_waits(me.alliance) then
        me:set_action(me.action - 4)
        return
    end
    me:set_action(me.action + 4)
end

function controller.make(spec: Spec): Controller
    local function warp(me: Object, out: boolean)
        local w = spec.warp
        if not w then
            error(string.format("%s's controller warps its user, but its game names no warp", spec.name))
        end
        w(me, out)
    end

    -- The navi, with the damage plus the bonus; a chip of the block kept, if the game keeps it.
    local function bring(me: Object)
        local s = me.state :: State
        local damage = dimming_chips.damage_plus(me, s.bonus)
        if spec.spawn(me, me.related1 :: Object, damage) then
            s.busy = true
        end
        if spec.keep and controller.in_block(s.chip, spec.navi_block) then
            spec.keep(s.chip, me.element, damage)
        end
    end

    -- The course by phase.
    local function by_phase(me: Object, steps: { Step })
        local n = me.phase // 4 + 1
        local step = steps[n]
        if step == nil or me.phase % 4 ~= 0 then
            error(string.format("%s's controller: effect phase %#x reads past its table", spec.name, me.phase))
        end
        local function next()
            if n == #steps then
                me:set_action(UNDIM)
            else
                me.phase, me.phase_init = me.phase + 4, 0
            end
        end
        if step == "navi" then
            if me.phase_init == 0 then
                bring(me)
                me.phase_init = 4
            end
            if not (me.state :: State).busy then
                next()
            end
            return
        end
        local wait = step :: { wait: number, warp: ("out" | "in")? }
        if me.phase_init == 0 then
            if wait.warp then
                warp(me, wait.warp == "out")
            end
            me.timer = wait.wait
            me.phase_init = 4
        end
        if ran_out(me) then
            next()
        end
    end

    -- EXE4's short course (0x080E2B9C), by the init byte: 0 the warp out, 1 the wait, 2 the navi and the wait after.
    local function short(me: Object, course: Short)
        local init = me.phase_init
        if init == 2 then
            if (me.state :: State).busy or not ran_out(me) then
                return
            end
            me:set_action(UNDIM)
            warp(me, false)
            return
        end
        if init ~= 1 then
            warp(me, true)
            me.phase_init = 1
            me.timer = course.out
        end
        if not ran_out(me) then
            return
        end
        me.phase_init = 2
        bring(me)
        me.timer = course.after
    end

    local function effect(me: Object)
        if spec.steps then
            by_phase(me, spec.steps)
        elseif spec.short then
            short(me, spec.short)
        else
            error(string.format("%s's controller has no course", spec.name))
        end
    end

    local kind = new.kind {
        id = spec.id,
        pool = "effect",
        state = { chip = "chip", bonus = "u16", busy = "bool" },
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                dimming.begin(me)
            elseif l == "update" then
                local a = me.action
                local chip = (me.state :: State).chip
                if a == DIM then
                    dimming.dim_screen(me)
                elseif a == ANTI_NAVI then
                    controller.anti_navi(me, controller.in_block(chip, spec.navi_block))
                elseif a == NAME then
                    controller.name(me, chip, controller.in_block(chip, spec.navi_block))
                elseif a == EFFECT then
                    effect(me)
                elseif a == UNDIM then
                    dimming.undim_screen(me)
                else
                    error(string.format("%s's controller: action %#x reads past its table", spec.name, a))
                end
            else
                dimming.finish(me)
            end
        end,
        navi_left = controller.done,
    }

    -- The chip's `dimming` hook (the game's dimming spawner's entry for it): the controller, on the user's panel, of
    -- its side and facing, with the chip and its bonus.
    local function hook(user: Object, dspec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, dspec)
        if o then
            local s = o.state :: State
            s.chip = dspec.chip
            s.bonus = dspec.bonus
        end
        return o
    end

    return { kind = kind, hook = hook }
end

return controller
