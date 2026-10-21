--!strict
-- C-BOMB's capsule (the DS's attack object 0xCD, 0x02016144): Meddy's chemical capsule (10-15) on a panel of the other
-- side's. It comes with a sound and lies there (its collision a thing nothing hits that reaches everything, so whatever
-- comes on its panel touches it). Touched, it sets off 30 ticks later: a hit over the 3x3 around it (no spark,
-- hitting navis, hit modifier 0, paralyzing for 120 ticks), explosions there, a sound, and it goes. Once Meddy forgets
-- it (her slot emptied as she is done) it goes in a puff instead.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")
local statuses = require("../../rules/status")

local capsule = {}

type State = {
    -- Meddy, whose slot (+0x74) holds it (its +0x60).
    meddy: Object?,
}
type Holder = { capsule: Object? }

local SPRITE = asset.sprite("chemical-capsule")
local SOUND_PLACE = asset.sound("place")
local SOUND_BURST = asset.sound("hit-bomb-1")
-- Its coming (14 ticks), its fuse once touched (30 ticks).
local COMING, FUSE = 0xE, 0x1E
local LIE, TOUCHED = 0x4, 0x8
-- The explosions (the effect table's row 0 in its palette 3: `sub_801BD3C` with 0x00030000), on the ground; the puff
-- 20 pixels up.
local EXPLOSION = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }
local EXPLOSION_PALETTE = 3
local PUFF_Z = 0x14_0000

-- Its slot in Meddy is empty.
local function forgotten(me: Object): boolean
    local meddy = (me.state :: State).meddy
    return meddy ~= nil and (meddy.state :: Holder).capsule == nil
end

-- `sub_801BD3C` with region 0x0F: an explosion on each panel of the 3x3 around it that is on the field, by its side.
local function explosions(me: Object)
    local dir = panels.side_direction(me.alliance)
    for _, p in regions.square.panels or {} do
        local x, y = me.panel_x + p[1] * dir, me.panel_y + p[2]
        if field.valid(x, y) then
            local cx, cy = field.center(x, y)
            battle.effect(Vec3.new(cx, cy, 0), EXPLOSION, 0, EXPLOSION_PALETTE)
        end
    end
end

-- Action 0 (0x020161B0): its sound, 14 ticks, then it lies there (animation 1).
local function come(me: Object)
    if me.phase_init == 0 then
        battle.play_sound(SOUND_PLACE)
        me.timer = COMING
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me.anim = 1
        me:set_action(LIE)
    end
end

-- Action 4 (0x020161E0): forgotten, it goes in a puff; touched, its fuse.
local function lie(me: Object)
    local c = me.collision
    if forgotten(me) then
        c:set_region(nil)
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, int.i32(p.z + PUFF_Z)), effects.puff)
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
        me:set_action(TOUCHED)
    end
end

-- Action 8 (0x02016220): 30 ticks, then it bursts and goes, Meddy's slot emptied.
local function burst(me: Object)
    if me.phase_init == 0 then
        me.timer = FUSE
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    local h = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = 0,
        z = 0,
        region = regions.square,
        target = collision.hits_navis,
        self_type = collision.attack,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 0,
        status = statuses.paralyze_120,
    })
    if h then
        h.run_while_dimmed = true
    end
    explosions(me)
    battle.play_sound(SOUND_BURST)
    local meddy = (me.state :: State).meddy
    if meddy then
        (meddy.state :: Holder).capsule = nil
    end
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

local function tick(me: Object)
    local c = me.collision
    c:remove()
    local a = me.action
    if a == 0 then
        come(me)
    elseif a == LIE then
        lie(me)
    elseif a == TOUCHED then
        burst(me)
    else
        error(string.format("C-BOMB's capsule: action %#x reads past its table (0x020D8118)", a))
    end
    c:present()
end

capsule.kind = new.kind {
    id = "c-bomb/capsule",
    pool = "attack",
    state = { meddy = "object" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x02016158 (`sub_8011504` with 0x01001015, 0x000F0000): its sprite with its shadow, animation 0; its
            -- collision (nothing reaches it, it reaches everything, hit modifier 0); shown, facing its way, on its panel
            -- on the ground.
            local sprite = me.sprite
            sprite:load(SPRITE)
            sprite.shadow = "ground"
            me.anim, me.anim_loaded = 0, 0xFF
            if not me:create_collision() then
                error("C-BOMB's capsule: no collision slot (sub_8011504; the game reads through a null pointer)")
            end
            me:setup_collision(collision.nothing, collision.everything, 0)
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me.visible = true
            me:set_coordinates_from_panel()
            local p = me.pos
            me.pos = Vec3.new(p.x, p.y, 0)
            me:set_lifecycle("update")
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x02016280 (`sub_801155A`): a capsule of Meddy's on (x, y), null, with her damage word, of her side and facing; it
-- runs while dimmed.
function capsule.spawn(meddy: Object, x: number, y: number): Object?
    local o = battle.spawn(capsule.kind)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = 0
    o:set_damage_word(bit32.bor(meddy.damage, bit32.lshift(meddy.stamina, 16)))
    o.alliance, o.flip = meddy.alliance, meddy.flip
    o.related1 = meddy
    ;(o.state :: State).meddy = meddy
    o.run_while_dimmed = true
    return o
end

return capsule
