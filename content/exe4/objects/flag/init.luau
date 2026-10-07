--!strict
-- EXE4's flag (attack object #0x9C, 0x080E06AC): what its flag stages (the actor lists' entry type 7, 0x080068F4:
-- records 76 to 87) plant in each side's back corner, its HP numbered under it; when a flag's HP runs out its side
-- loses the round. EXE6's code (`sub_80D8C5C`, which no EXE6 stage places) but for its sheet (10-32), its NameID (0xFD,
-- its HP number's: EXE6's 0xCD, whose look it has none of here), and its HP check (0x080E07CE): a panel that burns
-- burns it first (0x0801309E), a holy panel is EXE4's type 9, and its hit sounds 0xC8 (EXE6's 0x85).

local collision = require("../rules/collision")

local flag = {}

local SPRITE = asset.sprite("flag")
local DAMAGE = asset.sound("damage")
-- Its breaking (0x080E078A: 0x6F).
local BROKEN = asset.sound("hit-bomb-0")
local HIT_MOD = 3

-- 0x080E07CE (its argument 0): this tick's damage (a burning panel's first), halved rounding up on a holy panel, off
-- its HP: -1 when that leaves none (its HP 0), else 0, its sprite white while hit, with its hit's sound.
local function take_damage(me: Object): number
    me:burn_on_panel()
    local c = me.collision
    local total = 0
    for e = 0, 4 do
        total += c:element_damage(e)
    end
    local p = field.panel(me.panel_x, me.panel_y)
    if p and p.kind == "holy" then
        total = (total + 1) // 2
    end
    local hp = me.hp - total
    if hp <= 0 then
        me.hp = 0
        return -1
    end
    me.hp = hp
    me.sprite.white = total ~= 0
    if total ~= 0 then
        battle.play_sound(DAMAGE)
    end
    return 0
end

-- 0x080E0740: before the fight, nothing (0x0801466A hides it where the battle's settings byte 6 has 0x20: no
-- netbattle's, 0x1F); in it, it no longer runs while paused, shows, and takes this tick's hits; broken (unless time is
-- up), a sound and its side loses the round, and it goes.
local function tick(me: Object)
    if not battle.fighting() then
        return
    end
    me.run_while_paused = false
    me.visible = true
    me.collision:remove()
    if not battle.time_up() and take_damage(me) < 0 then
        battle.play_sound(BROKEN)
        battle.lose_round(me.alliance)
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    me:update_sprite()
    me.collision:present()
end

-- 0x080E06CC: on its panel, of no element, its HP numbered, its collision on the panel (rows 0x29 and 20), and its
-- first tick at once.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    me:set_coordinates_from_panel()
    me.visible = true
    me.element = 0
    -- (`sprite_setFlip(object_getFlip())`: EXE4's flip, 0x0800ACA6, is the object's side: side 1's flag points left.)
    sprite:set_flip(me.alliance)
    battle.show_hp(me, {})
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.collision_29, collision.hit_by_other_side, HIT_MOD)
    me.collision:present()
    me.lifecycle = "update"
    tick(me)
end

flag.kind = new.kind {
    id = "flag",
    pool = "attack",
    -- 0x080E06AC.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
    -- 0x080068F4: a flag stage's flag (an actor list's entry type 7), its argument's bit 7 its side and the rest its
    -- HP in eights; running while the battle is paused or dimmed. (0x080E07AA: until its init places it, its position
    -- is the spawner's registers, the panel and the HP.)
    place = function(spec: PlaceSpec): Object?
        local hp = bit32.band(spec.argument, 0x7F) * 8
        local o = battle.spawn(flag.kind, Vec3.new(spec.panel_x, spec.panel_y, hp))
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = spec.panel_x, spec.panel_y
        o.hp, o.max_hp = hp, hp
        o.alliance = bit32.rshift(spec.argument, 7)
        o.run_while_paused = true
        o.run_while_dimmed = true
        return o
    end,
}

return flag
