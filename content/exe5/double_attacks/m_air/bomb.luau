--!strict
-- M-AIR's bomb (the DS's attack object 0xA1, 0x02094B74; the GBA's attack object #0x62, 0x080D2284, which nothing
-- spawns, has the sprite and its sound): a magnet bomb GyroMan drops on a panel of the other side's as he flies over its
-- column. It falls from 40 pixels up, 2 a tick; from 20 pixels down it reaches its panel (region 0x01), thrown, hitting
-- navis, hit modifier 0, no spark, paralyzing (status 0x11), Elec (its spawner's element, 3); a hit ends it, and so
-- does the ground.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local status = require("../../rules/status")

local bomb = {}

local SPRITE = asset.sprite("magnet-bomb")
-- Its height, its fall a tick, and the height it reaches its panel from (16.16).
local HEIGHT, FALL, REACHES = 0x28_0000, 0x2_0000, 0x14_0000
-- The DS's spawner's element (0x020A5A06): Elec.
local ELEC = 3

-- 0x02094BC8: the hits against it resolve and spark; hit, it goes (hidden, no region); else from 20 pixels down its
-- panel is its region, and it falls; below the ground it goes. Its registration presents either way.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local gone = c.hit_flags ~= 0
    if not gone then
        local z = me.pos.z
        if z <= REACHES then
            c:set_region(regions.single)
        end
        z -= FALL
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y, z)
        gone = z < 0
    end
    if gone then
        me.visible = false
        c:set_region(nil)
        -- (The word store: the destroy state from its first action.)
        me:set_lifecycle("destroy")
    end
    c:present()
end

-- 0x02094B88: on its panel, 40 pixels up; its sprite (10-2E, no shadow, animation 0) and collision (thrown, hitting
-- navis, hit modifier 0: `sub_8011504`), shown; paralyzing; no region yet. Then its first tick at once.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, HEIGHT)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    -- (The DS doesn't check the collision it asks for; without one it would write through a null pointer.)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, 0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    local c = me.collision
    c.status_base = status.paralyze_120
    c:set_region(nil)
    c:present()
    me:set_lifecycle("update")
    tick(me)
end

bomb.kind = new.kind {
    id = "m-air/bomb",
    pool = "attack",
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            -- object_genericDestroy
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x02094C10: a bomb of `owner`'s side and facing on (x, y), Elec, with the damage word, running while dimmed.
function bomb.drop(owner: Object, x: number, y: number, damage: number): Object?
    local o = battle.spawn(bomb.kind)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = ELEC
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        o.run_while_dimmed = true
    end
    return o
end

return bomb
