--!strict
-- MetaGel's gel (attack object #0x45: EXE5's 0x080CE02C, EXE4's 0x080D5DF4; EXE6's `sub_80CCA40`, which no EXE6 chip
-- drops), made for a game's look (`gel.make`): it falls from 256 pixels up onto its panel, 8 a tick, and lands as an
-- aqua hit (hit modifier 3, the game's aqua spark); on a panel of the other side's area that is solid and free it
-- stays, taking the panel for its own side where its game lets it (`look.takes`), and goes after its splash and a
-- second; elsewhere it goes at once in a puff. It tells its controller (the holder) when it is gone. The games'
-- differ in their sounds, their puff's row and where the gel takes the panel: EXE5's where the other side may lose
-- it (0x0800BBCC), EXE4's anywhere but the other side's back column (0x080D5F0C).

local panels = require("@exelib/panels")
local regions = require("@exelib/regions")

local gel = {}

export type State = {
    -- Its controller and its place in the controller's list (the byte at ExtraVars it clears).
    holder: Object?,
    slot: number,
}

export type Look = {
    -- The kind's id.
    id: string,
    -- Its sprite (0c-26 in both games).
    sprite: SpriteAsset,
    -- Its sounds as it drops (its init) and as it lands.
    drop: Sound,
    land: Sound,
    -- The game's aqua spark, its collision type and what it hits (the navis).
    spark: Spark,
    is: CollisionType,
    hits: CollisionType,
    -- Whether a gel of side `side` that stays on (x, y) takes the panel.
    takes: (x: number, y: number, side: number) -> boolean,
    -- The puff it goes in (EXE5's effect row 0x14, EXE4's 0x15: the puff, animation 2), 16 pixels up.
    puff: Effect,
}

export type Gel = {
    kind: Kind,
    -- A gel on (x, y) for `holder`'s slot `slot`, of damage word `damage` (EXE5's 0x080CE212, EXE4's 0x080D5FD6). (Its
    -- position is the spawner's registers until its first update.)
    spawn: (holder: Object, x: number, y: number, damage: number, slot: number) -> Object?,
}

local DROP, START_Z = 0x80000, 0x100_0000
local HIT_MOD = 3
-- Where it stays (EXE5's 0x080CE1AC, EXE4's 0x080D5F70, by side): a solid panel of the other side's, with nothing on
-- it.
local function stays(x: number, y: number, side: number): boolean
    if side == 0 then
        return field.check(x, y, 0x30, 0x0F88_0080)
    end
    return field.check(x, y, 0x10, 0x0F88_00A0)
end
-- Where it may take the panel (EXE5's 0x080CE19C, EXE4's 0x080D5F60, by side).
local function may_take(x: number, y: number, side: number): boolean
    if side == 0 then
        return field.check(x, y, 0x20, 0x0F88_0080)
    end
    return field.check(x, y, 0x00, 0x0F88_00A0)
end
-- A taken panel stays its side's this long (`object_setPanelAllianceTimerLong`).
local STOLEN_TICKS = 0x708
local STAY_TICKS = 60

-- Its controller's list: it is gone.
local function gone(me: Object)
    local s = me.state :: State
    local holder = s.holder
    if holder and holder.active then
        local hs = holder.state :: { out: { boolean } }
        hs.out[s.slot] = false
    end
end

-- EXE5's where the other side may lose the panel (0x0800BBCC).
function gel.can_take(x: number, y: number, side: number): boolean
    return panels.can_take(x, y, bit32.bxor(side, 1))
end

function gel.make(look: Look): Gel
    local function puff(me: Object)
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), look.puff)
    end

    -- EXE5's 0x080CE0E0: fall; landed, its hit, and it stays or goes.
    local function fall(me: Object)
        local p = me.pos
        local z = int.i32(p.z - me.vel.z)
        me.pos = Vec3.new(p.x, p.y, z)
        if z > 0 then
            return
        end
        me.pos = Vec3.new(p.x, p.y, 0)
        battle.play_sound(look.land)
        local x, y = me.panel_x, me.panel_y
        local h = battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = 2,
            z = 0,
            region = regions.single,
            hit_effect = look.spark,
            target = look.hits,
            self_type = look.is,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = HIT_MOD,
        })
        if h then
            h.run_while_dimmed = true
        end
        if not stays(x, y, me.alliance) then
            puff(me)
            gone(me)
            me:free()
            return
        end
        if may_take(x, y, me.alliance) and look.takes(x, y, me.alliance) then
            field.set_alliance(x, y, me.alliance)
            field.set_column_timer(x, STOLEN_TICKS)
        end
        me.action = 4
    end

    -- EXE5's 0x080CE0A4: the battle over, gone; else its action.
    local function tick(me: Object)
        if battle.over() then
            gone(me)
            -- (EXE5's 0x080CE0C6: the state byte alone.)
            me.lifecycle = "destroy"
            return
        end
        local a = me.action
        if a == 0 then
            fall(me)
            if not me.active then
                return
            end
        elseif a == 4 then
            -- EXE5's 0x080CE1BC: its splash.
            me.anim = 1
            me.action = 8
        elseif a == 8 then
            -- EXE5's 0x080CE1C8: then it sits for a second.
            if bit32.btest(me.sprite.frame_flags, 0x80) then
                me.anim = 2
                me.timer = STAY_TICKS
                me.action = 12
            end
        elseif a == 12 then
            -- EXE5's 0x080CE1E2: gone in a puff.
            local t = me.timer - 1
            me.timer = t
            if t == 0 then
                gone(me)
                puff(me)
                -- (EXE5's 0x080CE206: the state byte alone.)
                me.lifecycle = "destroy"
            end
        else
            error(string.format("MetaGel's gel action %#x reads past its table", a))
        end
        me:update_sprite()
    end

    local kind = new.kind {
        id = look.id,
        pool = "attack",
        state = { holder = "object", slot = "u8" },
        -- EXE5's 0x080CE02C.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                -- EXE5's 0x080CE04C: high over its panel.
                me.visible = true
                local sprite = me.sprite
                sprite:load(look.sprite)
                sprite.shadow = "with_sprite"
                sprite:set_flip(bit32.bxor(me.alliance, me.flip))
                me.anim, me.anim_loaded = 0, 0xFF
                me.vel = Vec3.new(me.vel.x, me.vel.y, DROP)
                local cx, cy = field.center(me.panel_x, me.panel_y)
                me.pos = Vec3.new(cx, cy, START_Z)
                battle.play_sound(look.drop)
                me:set_lifecycle("update")
                tick(me)
            elseif l == "update" then
                tick(me)
            else
                me:free()
            end
        end,
    }

    local function spawn(holder: Object, x: number, y: number, damage: number, slot: number): Object?
        local o = battle.spawn(kind, Vec3.zero)
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

    return { kind = kind, spawn = spawn }
end

return gel
