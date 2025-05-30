#![allow(dead_code)]
use std::collections::HashMap;

use lazy_static::lazy_static;
use light_compressed_account::TreeType;
use solana_pubkey::{pubkey, Pubkey};

#[derive(Debug, Clone)]
pub struct TreeInfo {
    pub tree: Pubkey,
    pub queue: Pubkey,
    pub height: u32,
    pub tree_type: TreeType,
}

impl TreeInfo {
    pub fn get(pubkey: &str) -> Option<&TreeInfo> {
        QUEUE_TREE_MAPPING.get(pubkey)
    }

    pub fn height(pubkey: &str) -> Option<u32> {
        QUEUE_TREE_MAPPING.get(pubkey).map(|x| x.height)
    }
}

// TODO: keep updated with new trees. We could put it into a separate crate.
lazy_static! {
    pub static ref QUEUE_TREE_MAPPING: HashMap<String, TreeInfo> = {
        let legacy_state_trees = [
            (
                pubkey!("smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT"),
                pubkey!("nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148"),
            ),
            (
                pubkey!("smt2rJAFdyJJupwMKAqTNAJwvjhmiZ4JYGZmbVRw1Ho"),
                pubkey!("nfq2hgS7NYemXsFaFUCe3EMXSDSfnZnAe27jC6aPP1X"),
            ),
            (
                pubkey!("smt3AFtReRGVcrP11D6bSLEaKdUmrGfaTNowMVccJeu"),
                pubkey!("nfq3de4qt9d3wHxXWy1wcge3EXhid25mCr12bNWFdtV"),
            ),
            (
                pubkey!("smt4vjXvdjDFzvRMUxwTWnSy4c7cKkMaHuPrGsdDH7V"),
                pubkey!("nfq4Ncp1vk3mFnCQ9cvwidp9k2L6fxEyCo2nerYD25A"),
            ),
            (
                pubkey!("smt5uPaQT9n6b1qAkgyonmzRxtuazA53Rddwntqistc"),
                pubkey!("nfq5b5xEguPtdD6uPetZduyrB5EUqad7gcUE46rALau"),
            ),
            (
                pubkey!("smt6ukQDSPPYHSshQovmiRUjG9jGFq2hW9vgrDFk5Yz"),
                pubkey!("nfq6uzaNZ5n3EWF4t64M93AWzLGt5dXTikEA9fFRktv"),
            ),
            (
                pubkey!("smt7onMFkvi3RbyhQCMajudYQkB1afAFt9CDXBQTLz6"),
                pubkey!("nfq7yytdKkkLabu1KpvLsa5VPkvCT4jPWus5Yi74HTH"),
            ),
            (
                pubkey!("smt8TYxNy8SuhAdKJ8CeLtDkr2w6dgDmdz5ruiDw9Y9"),
                pubkey!("nfq8vExDykci3VUSpj9R1totVst87hJfFWevNK4hiFb"),
            ),
            (
                pubkey!("smt9ReAYRF5eFjTd5gBJMn5aKwNRcmp3ub2CQr2vW7j"),
                pubkey!("nfq9KFpNQL45ppP6ZG7zBpUeN18LZrNGkKyvV1kjTX2"),
            ),
            (
                pubkey!("smtAvYA5UbTRyKAkAj5kHs1CmrA42t6WkVLi4c6mA1f"),
                pubkey!("nfqAroCRkcZBgsAJDNkptKpsSWyM6cgB9XpWNNiCEC4"),
            ),
            (
                pubkey!("smtBvnJx2B2u85wc3sMkF6G8rVMfN8Ek3nVKZ8gQUFn"),
                pubkey!("nfqB3FAiiB1p3ksiWHB48LzSycpaJZ5RTp5C8RtNyUH"),
            ),
            (
                pubkey!("smtCEeVJsWyeeawgn5cQR5iK7dsJwnxJq7QwdQUepx8"),
                pubkey!("nfqC5pX1HzaTgUApL2DTp7Xh8j3A5Augk42jngRCoKF"),
            ),
            (
                pubkey!("smtF9XTNZeyMgGQxxWfxyS1Ff6CA4W4RgYi8X1wWxa9"),
                pubkey!("nfqFa5ZzBYELWDnMQZe7SA3gd1x98aqtPf4sfaJZQJm"),
            ),
            (
                pubkey!("smtGeMYXeGoyQVcnrDg985h74ak9aRPW4gsfdW25DVy"),
                pubkey!("nfqGKBHxkUbDvTtkiDXNWskBhM6R9YfCeNu52baqvaf"),
            ),
            (
                pubkey!("smtHxHypFJoK6z6CCgx7eP9jqDykUBE7PbrXrTVoejR"),
                pubkey!("nfqHEE21vgXLnD7wxauCvX6pfeAs1zJbE4YyZ4YQ1rG"),
            ),
            (
                pubkey!("smtJsXesAF3vEc7Kz86rvaaHnNndvRWRfTj3XhgbCyb"),
                pubkey!("nfqJnTp7kgAa2AF2QTRi5qNVinkpAdA15gBYYqeZUgA"),
            ),
            (
                pubkey!("smtKAoGiqSb6YwGhCSwsJer5tMMgk7sH1a2K5BNeNQQ"),
                pubkey!("nfqKejGFuD6xkNLt8zzp2HaypMeRDsptBaeVGB4Utoq"),
            ),
            (
                pubkey!("smtLdHZPfJfqK3cKCQ9sqQTCQaoDgZKA11MQZ9P4UFR"),
                pubkey!("nfqLk1L9ezj8AbDyeQueeQoKUvU6Jzz9eQs28QgTEfx"),
            ),
            (
                pubkey!("smtNKu3Dwsyw4YVVA7S9cWYGvLrwUVD3T593ZJnyggv"),
                pubkey!("nfqNG4bDC6e8SzamFhvDytxwzKdzbwoTsLHZFi11AD1"),
            ),
            (
                pubkey!("smta2xk2kZTeFBRzpSrtCpwmxkrQpv7LGgut1aMNsme"),
                pubkey!("nfqa2szxnkgX4xBTVG81HYK7mzZe8pSF8wv2yMXaTTG"),
            ),
            (
                pubkey!("smtb2BcLRWygF3svygXMprcRjXKUDnxvNFnseNgH6VT"),
                pubkey!("nfqbgaRZGC1BGtFjRMvJmx79fzg8bBuSJBCEbJzoGTG"),
            ),
            (
                pubkey!("smtd3wjo4AzEKd9tRE2zTanxEEWRAXAAs9AtF9NcfAs"),
                pubkey!("nfqd5yiNJJ5mvZxitwXY9bR5dfBs2WNcTKctFBYwSuv"),
            ),
            (
                pubkey!("smte57v68vyf21wT5xzxYvZpr6iiFG1WQ5dX7J1Y85E"),
                pubkey!("nfqecsLrkXwRpdBJZEpR2bJYbXc2jrh78mqg1kRDZKm"),
            ),
            (
                pubkey!("smtobNxYYVi8YfJDjzdoW1jR7xyZaVeXwmSHNgL3tA1"),
                pubkey!("nfqoqboretu8sLtCB4mTe3HKRmzc18HAPUAkEn18axG"),
            ),
            (
                pubkey!("smtpQZk7YARxMaz7VeW7zPMLNJAhbP9v1AZzLopaB2M"),
                pubkey!("nfqp7yDaPgGenuaFFAogXLvy5A5c3Znn5pYe6TmQ9RQ"),
            ),
            (
                pubkey!("smtqHbhmXHjVxeDNq5NPTMBw92L2ZsEF4q2WgNqjN7Y"),
                pubkey!("nfqqqib2xCHLXSVABHoczoY4u495T5eFCcypZ6C22gB"),
            ),
            (
                pubkey!("smtrG9ekG1obtqBRoB4mMUEwicfjTRRzZUm3z4LX8UJ"),
                pubkey!("nfqroTsZ4EX37MuYb26Km8nPmS2WhfG3HTFgCuuwe7U"),
            ),
            (
                pubkey!("smtsAZefsicmjKXz9Wtzidwt67pU3kqbhB6f2yD3rQJ"),
                pubkey!("nfqs5Hdbd7oKtDdRmVQFy4wytRn5gDb1DPwPyQCmHS2"),
            ),
            (
                pubkey!("smtt9Ra1v3mu8eSx7nrq5Q8bRqqPRf5mfpUvkpkP29L"),
                pubkey!("nfqt3kLwwcAm8wLfNCVGPThN7fpHimPoiBegoGeRxUy"),
            ),
            (
                pubkey!("smtu3VAWgucXQmMhy4S8nNojpuVJHgVrGQFkai1jXRw"),
                pubkey!("nfqu1jDCGChJQxQpU5XWjeHUtzYWBEoKZ24VXXdKdkk"),
            ),
            (
                pubkey!("smtvbupk8wjpXa48Zg29SVtTL8BpSJVrc6tfMGAA5A3"),
                pubkey!("nfqvcYyr6TzAugHSaX398fXPBSRygmb7TfmXoXvL8Qu"),
            ),
            (
                pubkey!("smtwntNZBnj3w5dw1mYjzgHBBhxAYvHjZhh5whVEaBS"),
                pubkey!("nfqw14GHxV2LJsNwwXLGCXDyQXqnUn6GDL9DKqBbeep"),
            ),
            (
                pubkey!("smtx7SjhPmjChWsUNiyZ4VF2U82zSBDf2yArGKr5BDb"),
                pubkey!("nfqxAGA7bDoHDxqA4K25fV1wZZ5NHzGrxReiCC5Ztet"),
            ),
            (
                pubkey!("smty1QArd6Z73H67TvoqpxitEc2E5A9zBtw42ZKZJkn"),
                pubkey!("nfqy55aAqL8qG5qBRixUtLnDqNd61ft2jtXyoYGHNGb"),
            ),
            (
                pubkey!("smtz1CZdRkGuMpYPZHihP2WruMj9ZHYjK6Ag9gLBzWM"),
                pubkey!("nfqzF2r8viCVTMpzVAL5jHVKsGF45RsASxun8ZpRKnm"),
            ),
        ];

        let address_trees_v1 = [(
            pubkey!("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2"),
            pubkey!("aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F"),
        )];

        let mut m = HashMap::new();

        for (legacy_tree, legacy_queue) in legacy_state_trees.iter() {
            m.insert(
                legacy_queue.to_string(),
                TreeInfo {
                    tree: *legacy_tree,
                    queue: *legacy_queue,
                    height: 26,
                    tree_type: TreeType::StateV1,
                },
            );

            m.insert(
                legacy_tree.to_string(),
                TreeInfo {
                    tree: *legacy_tree,
                    queue: *legacy_queue,
                    height: 26,
                    tree_type: TreeType::StateV1,
                },
            );
        }

        for (legacy_tree, legacy_queue) in address_trees_v1.iter() {
            m.insert(
                legacy_queue.to_string(),
                TreeInfo {
                    tree: *legacy_tree,
                    queue: *legacy_queue,
                    height: 26,
                    tree_type: TreeType::AddressV1,
                },
            );

            m.insert(
                legacy_tree.to_string(),
                TreeInfo {
                    tree: *legacy_tree,
                    queue: *legacy_queue,
                    height: 26,
                    tree_type: TreeType::AddressV1,
                },
            );
        }

        m.insert(
            "6L7SzhYB3anwEQ9cphpJ1U7Scwj57bx2xueReg7R9cKU".to_string(),
            TreeInfo {
                tree: pubkey!("HLKs5NJ8FXkJg8BrzJt56adFYYuwg5etzDtBbQYTsixu"),
                queue: pubkey!("6L7SzhYB3anwEQ9cphpJ1U7Scwj57bx2xueReg7R9cKU"),
                height: 32,
                tree_type: TreeType::StateV2,
            },
        );

        m.insert(
            "HLKs5NJ8FXkJg8BrzJt56adFYYuwg5etzDtBbQYTsixu".to_string(),
            TreeInfo {
                tree: pubkey!("HLKs5NJ8FXkJg8BrzJt56adFYYuwg5etzDtBbQYTsixu"),
                queue: pubkey!("6L7SzhYB3anwEQ9cphpJ1U7Scwj57bx2xueReg7R9cKU"),
                height: 32,
                tree_type: TreeType::StateV2,
            },
        );

        m.insert(
            "EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK".to_string(),
            TreeInfo {
                tree: pubkey!("EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK"),
                queue: pubkey!("EzKE84aVTkCUhDHLELqyJaq1Y7UVVmqxXqZjVHwHY3rK"),
                height: 40,
                tree_type: TreeType::AddressV2,
            },
        );

        m
    };
}
