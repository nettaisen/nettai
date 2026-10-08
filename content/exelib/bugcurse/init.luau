--!strict
-- BugCurse (EXE4's dimming chips' variant 0x2A and EXE5's effect object #0x56, the same code: EXE4's controller
-- 0x080E81C4 and curse 0x080DD39C, EXE5's 0x080E6AC8 and 0x080D6534), made for a game's look (`bugcurse.make`): a
-- controller (the usual dimming phases, @exelib/dimming) that lays a curse on the enemy navi's panel, unless the enemy
-- navi is out of its reach (the look's statuses) or there is none, and waits on it; then the undim. The curse (attack
-- object #0x80), shown with a sound, waits 59 ticks, then over 5 ticks hits its panel once a tick with a bug each of
-- its list (a one-tick breaking hit at navis, no spark, running while dimmed), and lets its controller go on.
--
-- The games part in the look: the kinds' ids, the curse's sheet and sound, what keeps the curse off (EXE5's adds the
-- bubble), the bugs (EXE4's four by code alone, EXE5's five with their arguments), and EXE5's operation battle (three
-- ticks of its own three bugs).

local dimming_chips = require("@exelib/dimming")
local regions = require("@exelib/regions")

local bugcurse = {}

type CurseState = {
    -- The controller waiting on it (its flag, +0x4C: the controller's +0x0C).
    controller: Object?,
}

type ControllerState = {
    -- The curse is still at work (+0x0C, the flag it was given).
    waiting: boolean,
}

-- Its wait before the bugs; their ticks.
local WAIT, TICKS, OPERATION_TICKS = 0x3A, 5, 3

-- A game's BugCurse: the chip's `dimming` hook.
function bugcurse.make(look: BugCurseLook): (user: Object, spec: DimmingChipSpec) -> Object?
    -- Its controller is done waiting on it (the flag it gave it, cleared).
    local function release(me: Object)
        local c = (me.state :: CurseState).controller
        if c then
            (c.state :: ControllerState).waiting = false
        end
    end

    -- EXE5's 0x080D6680 (EXE4's 0x080DD516): a one-tick hit on its panel carrying `bug` (code | argument << 8),
    -- running while dimmed.
    local function hit(me: Object, bug: number)
        local box = battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = me.element,
            z = 0,
            region = regions.single,
            hit_effect = nil,
            target = look.hits,
            self_type = look.is,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 0,
            bug = bit32.band(bug, 0xFF),
            bug_arg = bit32.rshift(bug, 8),
        })
        if box then
            box.run_while_dimmed = true
        end
    end

    -- Action 0 (EXE5's 0x080D65C8): 59 ticks (to -1).
    local function wait(me: Object)
        if me.phase_init == 0 then
            me.timer = WAIT
            me.phase_init = 4
        end
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left < 0 then
            me:set_action(4)
        end
    end

    -- Action 4 (EXE5's 0x080D65E8, EXE4's 0x080DD454): a bug a tick (EXE5's operation battle: its own, 3 ticks); then
    -- the controller moves on, and it goes.
    local function bugs(me: Object)
        local operation = look.operation_battle ~= nil and (look.operation_battle :: () -> boolean)()
        if me.phase_init == 0 then
            me.timer = if operation then OPERATION_TICKS else TICKS
            me.timer2 = 0
            me.phase_init = 4
        end
        local n = me.timer2
        if n < #look.bugs then
            local list = if operation then look.operation_bugs :: { number } else look.bugs
            local bug = list[n + 1]
            if bug == nil then
                error(string.format("BugCurse's curse: bug %d reads past its list", n))
            end
            hit(me, bug)
            me.timer2 = n + 1
        end
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left <= 0 then
            release(me)
            me:set_lifecycle("destroy")
        end
    end

    local ACTIONS = { [0] = wait, [4] = bugs }

    local function tick(me: Object)
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("BugCurse's curse: action %#x reads past its table", me.action))
        end
        action(me)
    end

    -- EXE5's 0x080D6558: its sprite (animation 0, a frame in, its first part drawn with it), shown, facing its way, on
    -- its panel; a sound; its first update.
    local function init(me: Object)
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "with_sprite"
        me.anim, me.anim_loaded = 0, 0xFF
        sprite:set_animation(0)
        sprite:step()
        me.visible = true
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me:set_coordinates_from_panel()
        battle.play_sound(look.sound)
        me:set_lifecycle("update")
        tick(me)
    end

    local curse = new.kind {
        id = look.curse_id,
        pool = "attack",
        state = { controller = "object" },
        -- EXE5's 0x080D6534.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
                return
            end
            me:update_sprite_while_dimmed()
        end,
    }

    -- EXE5's 0x080D6654: a curse on (x, y) with `element` and the damage word, of `controller`'s side, which waits on
    -- it; running while dimmed. (Its position is the spawner's registers: the row, the element and 0.)
    local function spawn(controller: Object, x: number, y: number, element: number, damage: number): Object?
        local o = battle.spawn(curse, Vec3.new(y, element, 0))
        if o then
            o.panel_x, o.panel_y = x, y
            o.element = element
            o:set_damage_word(damage)
            o.alliance = controller.alliance
            ;(o.state :: CurseState).controller = controller
            o.run_while_dimmed = true
        end
        return o
    end

    local function untouched(o: Object): boolean
        for _, f in look.untouched do
            if o:status(f) then
                return true
            end
        end
        return false
    end

    -- EXE5's 0x080E6B0C: on its first tick the curse (60 ticks on the timer, which nothing reads); then the undim once
    -- no curse is at work.
    local function effect(me: Object)
        local s = me.state :: ControllerState
        if me.phase_init == 0 then
            local enemy = battle.player(bit32.bxor(me.alliance, 1))
            if enemy and not untouched(enemy) then
                if spawn(me, enemy.panel_x, enemy.panel_y, me.element, bit32.bor(me.damage, bit32.lshift(me.stamina, 16))) then
                    s.waiting = true
                end
            end
            me.timer = 0x3C
            me.phase_init = 4
        end
        if not s.waiting then
            dimming_chips.done(me)
        end
    end

    local controller = new.kind {
        id = look.controller_id,
        pool = "effect",
        state = { waiting = "bool" },
        -- EXE5's 0x080E6AC8.
        update = dimming_chips.phases { name = "BugCurse", effect = effect },
    }

    -- EXE5's 0x080E6B62: BugCurse's `dimming` hook (only the alliance byte of its user's).
    return function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(controller, user, spec, true)
    end
end

return bugcurse
