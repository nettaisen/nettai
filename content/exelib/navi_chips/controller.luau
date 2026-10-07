--!strict
-- The navi chip controllers a game's dimming spawner makes (EXE4's navi chips: the dimming chips' action 0x0C, its
-- navi variants, each navi's effect object, 0x080E2B54 and its copies; EXE5's Phoenix and DethPhnx, its effect objects
-- #0x0D and #0x15): a navi chip's controller (EXE6's `sub_80E17E8`, the engine's) that its game runs as a dimming
-- chip's, with its own effect. Its actions: the dim, AntiNavi's check, the chip's name as a navi chip's
-- (`sub_800BA8A`), the effect, the undim. The effect is the game's course:
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
-- Each chip's `dimming` hook is its controller's `hook`; its navi tells the controller it is done through
-- `navi_chip.navi_left` (the kind's `navi_left`) or `controller.done`.

local dimming_chips = require("@exelib/dimming")

local controller = {}

-- The controller's state (`NaviChipControllerState`): the chip, which its telop names and AntiNavi checks (+0x30),
-- and its Atk+ bonus (+0x32), which the navi's damage adds; whether the navi is out (EXE5's +0x18, EXE4's +0x05 or
-- ExtraVars+0: the flag its spawn sets and its end clears); EXE4's: the chip's parameters the controller takes
-- (Param1, the navi's level: 0, an SP's 3, a DS's 4; Param3, which navi a shared controller brings), which its spawn
-- reads.
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
    -- How a side's navi warps out (`out`) or back in (EXE4's 0x080E2D26): where a course warps.
    warp: ((side: number, out: boolean) -> ())?,
}

-- What `make` hands back (`NaviChipController`): the kind, the chip's `dimming` hook, and the hook of a chip whose
-- parameters give the controller a level and a navi (EXE4's).
type Controller = NaviChipController

-- The actions: the dim, AntiNavi, the name, the effect, the undim.
local DIM, ANTI_NAVI, NAME, EFFECT, UNDIM = 0x0, 0x4, 0x8, 0xC, 0x10

-- The navi is done (it writes 0 through the pointer its spawn was given).
function controller.done(me: Object)
    (me.state :: State).busy = false
end

-- Count the timer down: true once it fell below 0.
local function ran_out(me: Object): boolean
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left < 0
end

function controller.make(spec: Spec): Controller
    local function warp(me: Object, out: boolean)
        local w = spec.warp
        if not w then
            error(string.format("%s's controller warps its user, but its game names no warp", spec.name))
        end
        w(me.alliance, out)
    end

    -- The navi, with the damage plus the bonus.
    local function bring(me: Object)
        local s = me.state :: State
        if spec.spawn(me, me.related1 :: Object, dimming_chips.damage_plus(me, s.bonus)) then
            s.busy = true
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
        state = { chip = "chip", bonus = "u16", busy = "bool", level = "u8", which = "u8" },
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
                    dimming.check_anti_navi(me, chip)
                elseif a == NAME then
                    dimming.show_navi_telop(me, chip)
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
    -- its side and facing, with the chip and its bonus, and the chip's parameters.
    local function hook_for(level: number, which: number?): (user: Object, spec: DimmingChipSpec) -> Object?
        return function(user: Object, dspec: DimmingChipSpec): Object?
            local o = dimming_chips.spawn(kind, user, dspec)
            if o then
                local s = o.state :: State
                s.chip = dspec.chip
                s.bonus = dspec.bonus
                s.level = level
                s.which = which or 0
            end
            return o
        end
    end

    return { kind = kind, hook = hook_for(0), hook_for = hook_for }
end

return controller
