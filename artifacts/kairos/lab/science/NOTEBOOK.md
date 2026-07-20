# KAIROS Adult Lab — Science Notebook (Season 3)

Parent lineage: `kairos-1783911389936799832`
Lab lineage: `kairos-1783911389936799832-lab`
Play mode: true
Love note reaffirmed: true
hypotheses total=20 accept=20 reject=0 inconclusive=0

## Latest day
- Self-heal: ACCEPT (w 0.7763 → damage 0.5513 → heal 0.7763)
- Multi-heal pack: ACCEPT
- Combined crisis heal: ACCEPT (w 0.7763->0.4908->0.7763)
- Self-improve: ACCEPT (util 0.8426 → 0.8426)
- Goal improve: ACCEPT goal_met=true
- Ternary: ACCEPT panel=`real_vcf chr22 snps=400 path=../data/raw/1000g/ALL.chr22.phase3_shapeit2_mvncall_integrated_v5b.20130502.genotypes.vcf.gz` (agree 100/100, speedup 12.17x)
- Ternary ladder: ACCEPT all_agree=true best_spd=12.17x rungs=3
- Self-mod single: skipped
- Self-mod readiness: ready=true

## Self-mod policy
Best after heal/improve/ternary streak. Lab-only, one cycle, then OFF.
Season 3 does **not** auto-run self-mod; use `--try-self-mod` when ready=true.

Primary cradle is **not** modified by this lab day.
