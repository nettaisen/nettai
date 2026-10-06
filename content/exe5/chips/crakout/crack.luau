--!strict
-- CrakOut, DublCrak and TripCrak (EXE5 chips 0x05B to 0x05D): their action (0x1B, 0x080EC79C, EXE5's own) and the
-- crack they leave (attack object 0x18, 0x080C7704: the code EXE6 keeps as `sub_80C7074`, no EXE6 chip uses it).
--
-- The navi points (animation 12) for 15 ticks; two ticks in, a crack appears on each panel of the chip's pattern
-- that is on the field (0x080C77C2, EXE6's `sub_80C7132`: by its subtype, the panel ahead; it and the one behind
-- it; it and the ones above and below), then 5 ticks to recover. A crack hits its panel once with the chip's
-- damage (no spark), breaks it (cracks it when something stands on it), and shows its dust for 19 ticks.
--
-- Its break is `object_breakPanelLoud` (EXE5's 0x0800B204): the panel break with its own sound (0xDA). Not as EXE5's:
-- the hit's element, which EXE5's crack leaves to a register (r2, what the sound call left: 0 here).

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")

local crack = {}

export type State = {
    -- The phase's countdown (AIAttackVars+0x10).
    timer: number,
}

local SPRITE = asset.sprite("dust")
local SOUND = asset.sound("bblstar1")
local BREAK_SOUND = asset.sound("crack-shot")

-- 0x080C7768 (EXE6's `sub_80C70D8`): the hit and the break, then 19 ticks.
local function tick(me: Object)
    if me.phase_init == 0 then
        battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = 0,
            z = 0,
            region = regions.single,
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.thrown,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 0,
        })
        field.break_panel(me.panel_x, me.panel_y, BREAK_SOUND)
        me.timer = 18
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me:set_lifecycle("destroy")
    end
end

-- 0x080C7728 (EXE6's `sub_80C7098`).
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    -- sprite_noShadow: the first part is drawn with the sprite.
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0xFF
    me:set_coordinates_from_panel()
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

crack.kind = new.kind {
    id = "crakout/crack",
    pool = "attack",
    -- 0x080C7704.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- 0x080C77A4 (EXE6's `sub_80C7114`): a crack of `owner`'s side on (x, y) with the damage word. (It spawns at the
-- registers' values: the panel Y, the element and Z, which its init replaces but for Z.)
local function spawn(owner: Object, x: number, y: number, element: number, damage: number)
    local o = battle.spawn(crack.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance = owner.alliance
        o:set_damage_word(damage)
    end
end

-- The action's phases, on the attack's step.
local POINT, RECOVER = 0x0, 0x4

-- The action of a chip that cracks the panels of `panels` (its pattern, a row of 0x080C7818 by its subtype: { dx, dy }
-- from the panel ahead, dx toward the navi's front).
function crack.action(panels: { { number } }): Action

    -- 0x080EC7B8: point; on the second tick, the cracks.
    local function point(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(12)
            me:open_counter_window()
            me:set_status("using_action", true)
            s.timer = 15
            me.step_init = 4
        end
        if s.timer == 13 then
            local dir = me.facing
            local x, y = me.panel_x + dir, me.panel_y
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            for _, p in panels do
                local px, py = x + dir * p[1], y + p[2]
                if field.valid(px, py) then
                    spawn(me, px, py, me.attack_element, damage)
                end
            end
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me.step = RECOVER
            me.step_init = 0
        end
    end

    -- 0x080EC814: 6 ticks, then back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = 5
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me:exit_attack()
        end
    end

    return new.action {
        state = { timer = "u16" },
        args = { panels = panels },
        -- 0x080EC79C.
        update = function(me: Object, s: State)
            local step = me.step
            if step == POINT then
                point(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("CrakOut's phase %#x reads past its table", step))
            end
        end,
    }
end

return crack
