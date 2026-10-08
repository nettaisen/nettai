--!strict
-- BurnMan's flame (attack object #0x37, 0x080D3EF0: EXE4's own), the one he breathes across the three panels ahead of
-- him: his sprite in its animation 8, no shadow, 8 pixels short of the panel ahead's center, and three lingering hits
-- (./navi's: fire, 60 ticks) on its panel and the two beyond, which it keeps in its `hits`; it highlights the three
-- for 60 ticks (59, counted from the next), then goes, its hits cut to their last tick. (It keeps itself in
-- BurnMan's +0x60 word too, which nothing reads but its own check whether it is still there: nothing else empties
-- it.)

local regions = require("@exelib/regions")

local flame = {}

type State = {
    -- Its lingering hits (its +0x60, +0x64, +0x68), which keep themselves here.
    hits: { Object? },
}

local SPRITE = asset.sprite("burnman")
local TICKS = 60

function flame.make(hits: (owner: Object, dx: number, dy: number, slot: number) -> ()): { kind: Kind, spawn: (Object, Vec3) -> Object? }
    -- 0x080D3F6A: (with the battle undimmed, gone at its end) its ticks; the three panels highlighted; then it goes,
    -- its hits' ticks cut to the last.
    local function tick(me: Object)
        local ended = not battle.dimmed() and battle.over()
        if not ended then
            local left = me.timer - 1
            me.timer = int.u16(left)
            if left > 0 then
                for _, o in regions.line.panels or {} do
                    field.highlight(me.panel_x + o[1] * me.facing, me.panel_y + o[2])
                end
                return
            end
        end
        me:set_lifecycle("destroy")
    end

    -- 0x080D3F20: his sprite in its animation 8, no shadow, shown, facing its side's way, on the panel its position
    -- is over; its three lingering hits (0x080D3FDC: its panel and the two ahead).
    local function init(me: Object)
        local sprite = me.sprite
        sprite:load(SPRITE)
        sprite.shadow = "with_sprite"
        me.visible = true
        me.anim, me.anim_loaded = 8, 8
        sprite:set_animation(8)
        me.timer = TICKS
        sprite.palette = 0
        sprite:set_flip(me.alliance)
        me:set_panel_from_coordinates()
        for i = 0, 2 do
            hits(me, i * me.facing, 0, i + 1)
        end
        me:set_lifecycle("update")
    end

    local kind = new.kind {
        id = "burnman/flame",
        pool = "attack",
        state = { hits = "object[3]" } :: StateSchema,
        -- 0x080D3EF0: by its state, then its sprite.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                -- 0x080D3F14: its hits' ticks cut to the last (0x080D4034), then it is freed.
                for _, h in (me.state :: State).hits do
                    if h then
                        h.timer = 1
                    end
                end
                me:free()
                return
            end
            me:update_sprite()
        end,
    }

    -- 0x080D3FC8: a flame of BurnMan's at `pos` (the panel ahead's center, 8 pixels back), fire, with his damage word,
    -- running while dimmed.
    local function spawn(burnman: Object, pos: Vec3): Object?
        local o = battle.spawn(kind, pos)
        if not o then
            return nil
        end
        o.element = 1
        o:set_damage_word(bit32.bor(burnman.damage, bit32.lshift(burnman.stamina, 16)))
        o.alliance = burnman.alliance
        o.run_while_dimmed = true
        return o
    end

    return { kind = kind, spawn = spawn }
end

return flame
