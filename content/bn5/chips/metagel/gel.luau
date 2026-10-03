--!strict
-- MetaGel's gel (BN5's attack object #0x45, 0x080CE02C; BN6's #0x45, `sub_80CCA40`, which no BN6 chip drops): it
-- falls from 256 pixels up onto its panel, 8 a tick, and lands as an aqua hit (BN5's rows 0xA and 5, hit modifier 3,
-- the aqua spark); on a panel of the other side's area that is solid and free it stays, taking the panel for its own
-- side where that side may lose it, and goes after its splash and a second; elsewhere it goes at once in a puff. It
-- tells its controller (the holder) when it is gone.

local collision = require("../../rules/collision")
local panels = require("@common/panels")
local regions = require("@common/regions")
local sparks = require("../../lib/sparks")

local gel = {}

export type State = {
    -- Its controller and its place in the controller's list (the byte at ExtraVars it clears).
    holder: Object?,
    slot: number,
}

-- 0x080CE04C: sprite 0c-26, no shadow, animation 0.
local SPRITE = asset.sprite("bn5:meta-gel")
local DROP, START_Z = 0x80000, 0x100_0000
local SOUND_DROP, SOUND_LAND = asset.sound("bn5:drop"), asset.sound("bn5:gel-land")
local HIT_MOD = 3
-- Where it stays (0x080CE1AC, by side): a solid panel of the other side's, with nothing on it.
local STAYS = {
    [0] = { require = 0x30, forbid = 0x0F88_0080 },
    [1] = { require = 0x10, forbid = 0x0F88_00A0 },
}
-- Where it takes the panel (0x080CE19C, by side).
local TAKES = {
    [0] = { require = 0x20, forbid = 0x0F88_0080 },
    [1] = { require = 0x00, forbid = 0x0F88_00A0 },
}
-- A taken panel stays its side's this long (`object_setPanelAllianceTimerLong`).
local STOLEN_TICKS = 0x708
local STAY_TICKS = 60
-- Effect 0x14 (a puff), 16 pixels up.
local PUFF = define.effect { sprite = asset.sprite("bn5:puff"), anim = 2, palette = 0 }

-- Its controller's list: it is gone.
local function gone(me: Object)
    local s = me.state :: State
    local holder = s.holder
    if holder and holder.active then
        local hs = holder.state :: { out: { boolean } }
        hs.out[s.slot] = false
    end
end

local function puff(me: Object)
    local p = me.pos
    battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), PUFF)
end

-- 0x080CE0E0: fall; landed, its hit, and it stays or goes.
local function fall(me: Object)
    local p = me.pos
    local z = int.i32(p.z - me.vel.z)
    me.pos = Vec3.new(p.x, p.y, z)
    if z > 0 then
        return
    end
    me.pos = Vec3.new(p.x, p.y, 0)
    battle.play_sound(SOUND_LAND)
    local x, y = me.panel_x, me.panel_y
    local h = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = 2,
        z = 0,
        region = regions.single,
        hit_effect = sparks.aqua,
        target = collision.hits_navis,
        self_type = collision.thrown,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
    if h then
        h.run_while_dimmed = true
    end
    local stays = STAYS[me.alliance]
    if not field.check(x, y, stays.require, stays.forbid) then
        puff(me)
        gone(me)
        me:free()
        return
    end
    local takes = TAKES[me.alliance]
    if field.check(x, y, takes.require, takes.forbid) and panels.can_take(x, y, bit32.bxor(me.alliance, 1)) then
        field.set_alliance(x, y, me.alliance)
        field.set_column_timer(x, STOLEN_TICKS)
    end
    me.action = 4
end

-- 0x080CE0A4: the battle over, gone; else its action.
local function tick(me: Object)
    if battle.over() then
        gone(me)
        me:set_lifecycle("destroy")
        return
    end
    local a = me.action
    if a == 0 then
        fall(me)
        if not me.active then
            return
        end
    elseif a == 4 then
        -- 0x080CE1BC: its splash.
        me.anim = 1
        me.action = 8
    elseif a == 8 then
        -- 0x080CE1C8: then it sits for a second.
        if bit32.btest(me.sprite.frame_flags, 0x80) then
            me.anim = 2
            me.timer = STAY_TICKS
            me.action = 12
        end
    elseif a == 12 then
        -- 0x080CE1E2: gone in a puff.
        local t = me.timer - 1
        me.timer = t
        if t == 0 then
            gone(me)
            puff(me)
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("MetaGel's gel action %#x reads past its table", a))
    end
    me:update_sprite()
end

gel.kind = define.kind {
    id = "bn5:metagel/gel",
    pool = "attack",
    state = { holder = "object", slot = "u8" },
    -- 0x080CE02C.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080CE04C: high over its panel.
            me.visible = true
            local sprite = me.sprite
            sprite:load(SPRITE)
            sprite.shadow = "with_sprite"
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me.anim, me.anim_loaded = 0, 0xFF
            me.vel = Vec3.new(me.vel.x, me.vel.y, DROP)
            local cx, cy = field.center(me.panel_x, me.panel_y)
            me.pos = Vec3.new(cx, cy, START_Z)
            battle.play_sound(SOUND_DROP)
            me:set_lifecycle("update")
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

-- 0x080CE212: a gel on (x, y) for `holder`'s slot `slot`, of damage word `damage`. (Its position is the spawner's
-- registers until its first update.)
function gel.spawn(holder: Object, x: number, y: number, damage: number, slot: number): Object?
    local o = battle.spawn(gel.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        o:set_damage_word(damage)
        o.alliance = holder.alliance
        local s = o.state :: State
        s.holder, s.slot = holder, slot
        local hs = holder.state :: { out: { boolean } }
        hs.out[slot] = true
    end
    return o
end

return gel
