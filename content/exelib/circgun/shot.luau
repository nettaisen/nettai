--!strict
-- CircGun's shot (the original's attack object #0x8A, `sub_80D67EC`;
-- docs/engine/dimming-chip-effects.md §8): the gun's mark on a panel that,
-- 29 ticks later, hits it if it is solid, with a flash, and tells the gun
-- it is done. Its sprite steps while dimmed. Another game's shot is a kind
-- of its look (`shot.make_kind`, with the gun's: ./gun; EXE5's DarkCirc).

local regions = require("../regions")

local shot = {}

-- What the gun keeps of its shots (its state): one flag each.
type GunState = { shots_out: { number } }

export type State = {
    -- The gun waiting on it, and its flag there (ExtraVars+4 + i).
    gun: Object?,
    slot: number,
    -- Its gun's variant: its palette, what it hits as and the bug it carries.
    variant: CircGunVariant,
    ticks: number,
}

local ANIM = 1


-- `sub_80D67D2`: on its panel, 8 pixels down and up.
function shot.place(me: Object)
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y + 0x8_0000, 0x8_0000)
end

-- EXE5's (0x080D76F0, and its gun's place): on its panel, at height 0.
function shot.place_on_floor(me: Object)
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
end

-- A shot kind (`id`) of `look_`.
function shot.make_kind(id: string, look_: CircGunLook): Kind
    -- Done: the gun's flag cleared (as the original, wherever the gun was);
    -- freed next update.
    local function finish(me: Object)
        local s = me.state :: State
        local gun = s.gun
        if gun and gun.active then
            (gun.state :: GunState).shots_out[s.slot + 1] = 0
        end
        me:set_lifecycle("destroy")
    end

    -- `sub_80D6874`: 29 ticks.
    local function wait(me: Object)
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me.anim = ANIM
            me:set_action(4)
        end
    end

    -- `sub_80D688C`: hit the panel if it is solid.
    local function hit(me: Object)
        if field.solid(me.panel_x, me.panel_y) then
            local v = (me.state :: State).variant
            local h = battle.hitbox(me, {
                panel_x = me.panel_x,
                panel_y = me.panel_y,
                element = me.element,
                z = 0,
                region = regions.single,
                hit_effect = look_.spark,
                target = look_.target,
                self_type = v.hit,
                damage = me.damage,
                stamina = me.stamina,
                hit_mod = 3,
                bug = v.bug or 0,
                bug_arg = v.bug_arg or 0,
            })
            if h then
                h.run_while_dimmed = true
            end
            battle.effect(me.pos, look_.flash)
            battle.play_sound(look_.sounds.hit)
        end
        finish(me)
    end

    -- `sub_80D685A`.
    local function tick(me: Object)
        local a = me.action
        if a == 0 then
            wait(me)
        elseif a == 4 then
            hit(me)
        else
            error(string.format("CircGun's shot: action %#x reads past off_80D686C", a))
        end
    end

    -- `sub_80D6810`: the mark on its panel, in its look's palette.
    local function init(me: Object)
        local s = me.state :: State
        local sprite = me.sprite
        sprite:load(look_.sprite)
        -- sprite_noShadow: the first part is drawn with the sprite.
        sprite.shadow = "with_sprite"
        me.anim, me.anim_loaded = ANIM, 0xFF
        me.visible = true
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        if look_.on_floor then
            shot.place_on_floor(me)
        else
            shot.place(me)
        end
        me.timer = s.ticks
        sprite.palette = s.variant.palette
        battle.play_sound(look_.sounds.mark)
        me:set_lifecycle("update")
        tick(me)
    end

    return new.kind {
        id = id,
        pool = "attack",
        state = { gun = "object", slot = "u8", variant = "record:circgun-variant", ticks = "u8" },
        -- `sub_80D67EC`.
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
end

-- `sub_80D68E0`: a shot on (x, y) with `element` and the damage word, of
-- `gun`'s side (not its facing) and variant, running while dimmed, waiting
-- `ticks`; the gun's flag `slot` is set until it is done.
function shot.spawn_kind(
    kind: Kind,
    gun: Object,
    x: number,
    y: number,
    element: number,
    damage: number,
    ticks: number,
    variant: CircGunVariant,
    slot: number
): Object?
    local o = battle.spawn(kind)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o:set_damage_word(damage)
    o.alliance = gun.alliance
    local s = o.state :: State
    s.gun, s.slot, s.variant, s.ticks = gun, slot, variant, ticks
    ;(gun.state :: GunState).shots_out[slot + 1] = 1
    o.run_while_dimmed = true
    return o
end

return shot
