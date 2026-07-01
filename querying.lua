-- Per-table column widths, keyed by header text, since a single blanket ratio
-- breaks tables whose first column holds long unbreakable inline-code tokens
-- (e.g. "transitive_closure_query()") -- those need more room than a table
-- of short identifiers like "seeds"/"max_hops" does.
local PRESETS = {
  ["Field|Required|Description"] = {0.22, 0.10, 0.68},
  ["Primitive|Shape|Use when"] = {0.33, 0.27, 0.40},
}

function Table(tbl)
  if #tbl.head.rows == 0 then
    return tbl
  end
  local headers = {}
  for _, cell in ipairs(tbl.head.rows[1].cells) do
    table.insert(headers, pandoc.utils.stringify(cell))
  end
  local widths = PRESETS[table.concat(headers, "|")]
  if widths then
    for i, spec in ipairs(tbl.colspecs) do
      spec[2] = widths[i]
    end
  end
  return tbl
end
