#!/bin/sh


set -euxo pipefail

RB=./target/debug/rb

DRINK=${1-"drink.0eghfh5gl9ivhq5nnvfqq7krfo"}
order_id=$(${RB} dev-config.toml order ${DRINK})
${RB} dev-config.toml process-order 
${RB} dev-config.toml process-barista
${RB} dev-config.toml order-status ${order_id}
