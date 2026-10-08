--!strict
-- A rock falling on a panel, as EXE6 and EXE4 share it (EXE6's attack object #0x1D, `sub_80C7D78`; EXE4's #0x1D,
-- 0x080D08BC, the same code but for its constants): it appears high over its panel and falls, faster and faster; its
-- hit region opens once it is under 30 pixels. What it hits breaks it into two chips (@exelib/rock/falling_chip);
-- landing it breaks on a solid panel, or drops into a hole with a puff. EXE6's rules' GroundCross barrage spawns it
-- (`sub_80C7F20`), EXE4's GutsMan's quake (0x080D0A40), with the panel, element and damage word. (EXE6's
-- ExtraVars[0] may point at a counter of rocks in flight; the barrage passes none, and EXE4's has none.)
--
-- Each game makes its kind (`falling.make_kind`) with its look.

local regions = require("../regions")

local falling = {}

-- A game's falling rock: its kind's id; its sprite; the height it starts at and its fall's acceleration (16.16);
-- its collision (what it is, what it hits, its hit modifier, its spark: hit effect 0); the puff it drops into a hole
-- with; the chips it breaks into and their sound.
export type Look = {
    id: string,
    sprite: SpriteAsset,
    height: number,
    gravity: number,
    is: CollisionType,
    hits: CollisionType,
    hit_mod: number,
    spark: Spark,
    puff: Effect,
    chips: RockChips,
    shatter_sound: Sound,
}

export type RockChips = {
    spawn: (pos: Vec3, vx: number, vz: number, priority: number) -> Object?,
}

-- It can hit once lower than this, in whole pixels.
local HIT_HEIGHT = 0x1E

-- The chips a rock breaks into: their sideways and upward speeds (EXE6's `dword_80C7F00`, EXE4's 0x080D0A20), and
-- their priority.
local CHIPS = { { 0x8000, 0x28000 }, { -0x8000, 0x30000 } }
local CHIP_PRIORITY = 2

function falling.make_kind(look: Look): Kind
    -- EXE6's `sub_80C7EC8`, EXE4's 0x080D09E8: two chips fly off from its position at ground level (in front at
    -- priority 2), and its sound.
    local function shatter(me: Object)
        for i = 1, 2 do
            -- The chips' Param1 is a draw's low bits plus one (unread; the draw stays).
            battle.rng()
            local p = me.pos
            look.chips.spawn(Vec3.new(p.x, p.y, 0), CHIPS[i][1], CHIPS[i][2], CHIP_PRIORITY)
        end
        battle.play_sound(look.shatter_sound)
    end

    -- EXE6's `sub_80C7D98`, EXE4's 0x080D08DC: over its panel, falling from its height, with its hit registered
    -- (region closed until low enough). Not once time is up (the routine reads `battle_isBattleOver`'s Z flag, which
    -- a KO leaves clear) or when its panel is off the field: it is freed.
    local function init(me: Object): boolean
        if battle.time_up() or not field.valid(me.panel_x, me.panel_y) then
            me:free()
            return false
        end
        local x, y = field.center(me.panel_x, me.panel_y)
        me.pos = Vec3.new(x, y, look.height)
        me.visible = true
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "ground"
        me.anim = 0
        sprite:set_animation(0)
        sprite:set_flip(bit32.bxor(me.alliance, me.flip))
        me.vel = Vec3.new(me.vel.x, me.vel.y, 0)
        if not me:create_collision() then
            me:free()
            return false
        end
        me:setup_collision(look.is, look.hits, look.hit_mod)
        me.collision:set_hit_effect(look.spark)
        me.collision:set_region(nil)
        me.collision:present()
        me:set_lifecycle("update")
        return true
    end

    -- EXE6's `sub_80C7E6C`, EXE4's 0x080D0998: open the region once under 30 pixels (that tick it doesn't move);
    -- fall; at the ground, break or puff, and go.
    local function fall(me: Object)
        if me.phase == 0 and bit32.rshift(me.pos.z, 16) < HIT_HEIGHT then
            me.collision:set_region(regions.single)
            me.phase = 4
            me.phase_init = 0
            return
        end
        local vz = me.vel.z + look.gravity
        me.vel = Vec3.new(me.vel.x, me.vel.y, vz)
        local z = me.pos.z - vz
        me.pos = Vec3.new(me.pos.x, me.pos.y, z)
        if z > 0 then
            return
        end
        if field.solid(me.panel_x, me.panel_y) then
            shatter(me)
        else
            battle.effect(me.pos, look.puff)
        end
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end

    -- EXE6's `sub_80C7E24`, EXE4's 0x080D095C: resolve its hit (and the spark); a hit breaks it.
    local function update(me: Object)
        me.collision:remove()
        me:hit_spark()
        if me.collision.hit_flags ~= 0 then
            shatter(me)
            me.collision:set_region(nil)
            me:set_lifecycle("destroy")
            return
        end
        if me.action ~= 0 then
            error(string.format("%s: action %d runs off its table (sub_80C7E24)", look.id, me.action))
        end
        fall(me)
        me:update_sprite()
        me.collision:present()
    end

    return new.kind {
        id = look.id,
        pool = "attack",
        -- EXE6's `sub_80C7D78`, EXE4's 0x080D08BC.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                if init(me) then
                    update(me)
                end
            elseif l == "update" then
                update(me)
            else
                me:destroy()
            end
        end,
    }
end

return falling
