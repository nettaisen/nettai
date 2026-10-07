--!strict
-- CrakOut, DublCrak and TripCrak's action and the crack they leave, as EXE5 and EXE4 share them (EXE5's action 0x1B,
-- 0x080EC79C, and attack object 0x18, 0x080C7704: the code EXE6 keeps as `sub_80C7074`, no EXE6 chip uses it; EXE4's
-- action 0x21, 0x080ECA14, and attack object 0x18, 0x080CFE5C, the same code).
--
-- The navi points for 15 ticks; two ticks in, a crack appears on each panel of the chip's pattern (EXE5's spawner,
-- 0x080C77C2, EXE6's `sub_80C7132`, on the field's panels alone; EXE4's, 0x080CFF1A, on every one: its rows, by the
-- attack's variant, at 0x080CFF64), then 5 ticks to recover. A crack hits its panel once with the chip's damage (no
-- spark), breaks it with its own sound (cracks it when something stands on it), and shows its dust for 19 ticks. A
-- game gives the look (`crack.make`): the kind's id, the pointing animation (EXE5's 12, EXE4's 16), the dust, the
-- sounds, the collision rows, and whether the spawner keeps to the field. Not as the original's: the hit's element,
-- which the crack leaves to a register (r2, what its sound call left: 0 here).

local regions = require("@exelib/regions")

local crack = {}

export type Look = {
    -- The crack kind's id.
    id: string,
    -- The navi's pointing animation.
    anim: number,
    -- The crack's dust, its sound as it comes, and the break's sound in place of the panel crack's (EXE5's
    -- `object_breakPanelLoud`, 0x0800B204; EXE4's 0x08009D04).
    sprite: SpriteAsset,
    sound: Sound,
    break_sound: Sound,
    -- What the crack's hit is and reaches (the game's thrown and hits-navis rows).
    is: CollisionType,
    hits: CollisionType,
    -- The spawner places a crack only on a panel of the field (EXE5's), or on every panel of the pattern (EXE4's).
    on_field_only: boolean,
}

export type Crack = {
    kind: Kind,
    look: Look,
}

export type State = {
    -- The phase's countdown (AIAttackVars+0x10).
    timer: number,
}

-- A game's crack: its kind and look.
function crack.make(look: Look): Crack
    -- EXE5's 0x080C7768, EXE4's 0x080CFEC2 (EXE6's `sub_80C70D8`): the hit and the break, then 19 ticks.
    local function tick(me: Object)
        if me.phase_init == 0 then
            battle.hitbox(me, {
                panel_x = me.panel_x,
                panel_y = me.panel_y,
                element = 0,
                z = 0,
                region = regions.single,
                hit_effect = nil,
                target = look.hits,
                self_type = look.is,
                damage = me.damage,
                stamina = me.stamina,
                hit_mod = 0,
            })
            field.break_panel(me.panel_x, me.panel_y, look.break_sound)
            me.timer = 18
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me:set_lifecycle("destroy")
        end
    end

    -- EXE5's 0x080C7728, EXE4's 0x080CFE80 (EXE6's `sub_80C7098`).
    local function init(me: Object)
        local sprite = me.sprite
        sprite:load(look.sprite)
        -- sprite_noShadow: the first part is drawn with the sprite.
        sprite.shadow = "with_sprite"
        me.visible = true
        me.anim, me.anim_loaded = 0, 0xFF
        me:set_coordinates_from_panel()
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        battle.play_sound(look.sound)
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = new.kind {
        id = look.id,
        pool = "attack",
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
    return { kind = kind, look = look }
end

-- EXE5's 0x080C77A4, EXE4's 0x080CFEFC (EXE6's `sub_80C7114`): a crack of `owner`'s side on (x, y) with the damage
-- word. (It spawns at the registers' values: the panel Y, the element and Z, which its init replaces but for Z.)
local function spawn(c: Crack, owner: Object, x: number, y: number, element: number, damage: number)
    local o = battle.spawn(c.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance = owner.alliance
        o:set_damage_word(damage)
    end
end

-- The action's phases, on the attack's step.
local POINT, RECOVER = 0x0, 0x4

-- The action of a chip that cracks the panels of `panels` (its pattern, a row by its variant: { dx, dy } from the
-- panel ahead, dx toward the navi's front), with game `c`'s crack.
function crack.action(c: Crack, panels: { { number } }, id: string?): Action
    local look = c.look

    -- EXE5's 0x080EC7B8, EXE4's 0x080ECA34: point; on the second tick, the cracks.
    local function point(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(look.anim)
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
                if not look.on_field_only or field.valid(px, py) then
                    spawn(c, me, px, py, me.attack_element, damage)
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

    -- EXE5's 0x080EC814, EXE4's 0x080ECA90: 6 ticks, then back to idle.
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
        id = id,
        state = { timer = "u16" },
        args = { panels = panels },
        -- EXE5's 0x080EC79C, EXE4's 0x080ECA14.
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
