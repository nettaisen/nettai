--!strict
-- The chips sent this battle (0x0203DB70, their count at 0x020364A8, which the battle's start empties, 0x0801910C): a
-- console's list of the picks of every hand installed, either side's (0x08008916: side 0's block, then side 1's in a
-- netbattle, each through 0x08007E92, unfiltered), which JunkSoul's custom screen offers again
-- (navis/megaman/forms/junksoul/recycle: the list's one reader).
--
-- A hand's are its block's +0x32, written as OK builds it (0x0801F10A): its picks in order, but for this list's own
-- chips offered again (their entries' flags 2 and on, 0x0801F118) and a soul's chip (family 15), as they were picked:
-- before the Atk+ chips and a Program Advance fold them, so an Atk+10 is listed. A dark chip offered to a dark
-- MegaMan is a pick as any (its slot's flag 0x20 leaves its entry's flags alone, 0x0801EF5A), so it is sent too, and a
-- JunkSoul on either side may be offered it again: a dark chip's class is a standard chip's.
-- (The count wraps at 64, 0x08007EC2, which two folders of 30 never reach.)

local sent_chips = {}

type State = {
    sent: { FolderChip },
    sending: { FolderChip },
}

sent_chips.state = {
    -- A console's list.
    sent = schema.list({ chip = "chip", code = "code" }, 60),
    -- The side's hand's (its block's +0x32), from OK to the install.
    sending = schema.list({ chip = "chip", code = "code" }, 5),
} :: StateSchema

-- 0x0801F10A, as OK builds the side's hand: the picks its hand lists.
function sent_chips.confirmed(side: number)
    local sending: { FolderChip } = {}
    for _, slot in custom.picks(side) do
        local here = custom.slot(side, slot)
        if here ~= nil and here.listed then
            table.insert(sending, { chip = here.chip, code = here.code })
        end
    end
    (rules.state() :: State).sending = sending
end

-- 0x08008916, as the side's hand is installed: its chips go in each console's list.
function sent_chips.installed(side: number)
    local s = rules.state() :: State
    local sending = s.sending
    for console = 0, 1 do
        local sent = (rules.state_of(console) :: State).sent
        for i = 1, #sending do
            local c = sending[i]
            sent[#sent + 1] = { chip = c.chip, code = c.code }
        end
    end
    s.sending = {}
end

-- The side's console's list, in its order.
function sent_chips.of(side: number): { FolderChip }
    local sent = (rules.state_of(side) :: State).sent
    local out: { FolderChip } = {}
    for i = 1, #sent do
        local c = sent[i]
        out[i] = { chip = c.chip, code = c.code }
    end
    return out
end

-- ... and the list written (its entries in another order, or some gone).
function sent_chips.set(side: number, sent: { FolderChip })
    (rules.state_of(side) :: State).sent = sent
end

return sent_chips
