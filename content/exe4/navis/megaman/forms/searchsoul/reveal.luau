--!strict
-- SearchSoul's reveal (EXE4's effect object 0x44, 0x080E6D74; spawned by its status reset, 0x080E6E4C: EXE5's 0x8F
-- grew from it), which its status reset spawns: unseen, on its first update it marks the other side's navi when it is
-- invisible, submerged or flashing (flag1 0x206) with the scope's mark (effect row 0x52, 20 pixels up); on its next a
-- hit over the whole field reveals what it reaches: no damage, through invisibility (row 0x2C, at navis); then it
-- goes.
--
-- The slots it marks are the battle's actor list from the other side's (`sub_800A832`'s battle number 70 and on, a
-- netbattle: one slot, the other side's navi; under 70 three slots on, which no netbattle reaches).

local collision = require("../../../../rules/collision")
local regions = require("@exelib/regions")

local reveal = {}

-- The scope's mark (effect row 0x52: 10-23), 20 pixels up.
local MARK = new.effect { sprite = asset.sprite("scope-mark"), anim = 0, palette = 0 }
local MARK_Z = 20 * 0x1_0000
-- What it marks: an invisible, submerged or flashing navi.
local function hidden(navi: Object): boolean
    return navi:status("invisible") or navi:status("submerged") or navi:status("flashing")
end

-- 0x080E6DB4: the other side's hidden navi marked; the hit on the next update. (The timer of 10 it sets is read only
-- by the marks' second update, which the action of 4 leaves unrun.)
local function mark(me: Object)
    local navi = battle.player(bit32.bxor(me.alliance, 1))
    if navi ~= nil and hidden(navi) then
        battle.effect(navi.pos + Vec3.new(0, 0, MARK_Z), MARK)
    end
    me.timer = 10
    me.action, me.phase = 4, 4
end

-- 0x080E6E2C: the hit (attack 3, its parameters 0x2C05FF80: the whole field, no spark, at navis, row 0x2C; no
-- damage), anchored at panel (2, 2), running while dimmed (0x080CD81E: its header flag 0x10); then it goes.
local function hit(me: Object)
    local o = battle.hitbox(me, {
        panel_x = 2,
        panel_y = 2,
        element = 0,
        region = regions.field,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.drain,
        damage = 0,
    })
    if o then
        o.run_while_dimmed = true
    end
    me:set_lifecycle("destroy")
end

reveal.kind = new.kind {
    id = "searchsoul/reveal",
    pool = "effect",
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080E6D94
            me:set_lifecycle("update")
        elseif l == "update" then
            if me.action == 0 then
                if me.phase_init == 0 then
                    mark(me)
                else
                    local t = me.timer - 1
                    me.timer = bit32.band(t, 0xFFFF)
                    if t < 0 then
                        me:set_lifecycle("update")
                    end
                end
            elseif me.action == 4 then
                hit(me)
            else
                error(string.format("SearchSoul's reveal's action %#x reads past its table (0x080E6DAC)", me.action))
            end
        else
            me:free()
        end
    end,
}

-- 0x080E6E4C: the reveal of `navi`'s side (at the status reset's registers: an effect's position nothing reads).
function reveal.spawn(navi: Object): Object?
    local o = battle.spawn(reveal.kind)
    if o then
        o.alliance = navi.alliance
    end
    return o
end

return reveal
